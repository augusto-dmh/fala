//! Ditado longo: os blocos que o `DictationCapture` fecha durante a gravação são transcritos numa
//! thread de fundo, na ordem, e ao soltar só falta o resto. Sem bloco fechado, o ditado segue o
//! caminho de sempre (`TranscriptionManager::transcribe` no áudio inteiro).
//!
//! O motor do ASR atende uma chamada por vez (`transcribe` o tira do mutex), então a fila tem uma
//! thread só, o resto vai pela mesma fila, e depois de um cancelamento quem usa o motor de novo
//! espera o bloco que estava nele terminar.
//!
//! Com um formatador, o texto de cada bloco fechado vai a ele numa thread própria assim que sai do
//! ASR (o próximo bloco não espera o LLM). Ao soltar, quem chamou formata o resto e `compose` junta
//! os textos: o do LLM onde ele respondeu, o das regras onde não respondeu ("IA parcial").

use std::sync::{mpsc, Arc, Mutex, MutexGuard};
use std::thread::JoinHandle;

use anyhow::anyhow;
use fala_postproc::LateEdit;

use crate::actions::ProcessedTranscription;

/// Transcreve um bloco a 16 kHz mono.
pub type Transcribe = Arc<dyn Fn(Vec<f32>) -> anyhow::Result<String> + Send + Sync>;
/// Formata o texto de um bloco como o de um ditado curto (regras, dicionário, LLM com 2 s).
pub type Format = Arc<dyn Fn(String) -> ProcessedTranscription + Send + Sync>;

/// O que uma gravação faz com os blocos que fecham durante ela.
pub struct Blocks {
    pub transcribe: Transcribe,
    /// `None` deixa a formatação para o texto inteiro, depois do soltar.
    pub format: Option<Format>,
}

/// Um pedaço transcrito: o texto do ASR e, para um bloco fechado com formatador, a formatação
/// em curso. O resto do soltar nunca tem formatação.
pub struct Part {
    pub raw: String,
    pub formatting: Option<JoinHandle<ProcessedTranscription>>,
}

/// O ditado longo transcrito, e em quantos pedaços o ASR rodou.
pub struct Finished {
    pub parts: anyhow::Result<Vec<Part>>,
    pub blocks: usize,
}

impl Finished {
    /// Os textos do ASR juntados.
    pub fn raw(&self) -> anyhow::Result<String> {
        match &self.parts {
            Ok(parts) => Ok(join(parts.iter().map(|p| p.raw.as_str()))),
            Err(e) => Err(anyhow!("{e:#}")),
        }
    }
}

#[derive(Default)]
enum State {
    /// Esta gravação não usa blocos.
    #[default]
    Off,
    /// Usa, e nenhum bloco fechou ainda.
    Armed(Blocks),
    Running(Run),
}

type Done = (
    anyhow::Result<String>,
    Option<JoinHandle<ProcessedTranscription>>,
);

struct Run {
    /// Cada bloco, e se é o resto do soltar.
    jobs: mpsc::Sender<(Vec<f32>, bool)>,
    results: mpsc::Receiver<Done>,
    worker: JoinHandle<()>,
    blocks: usize,
    samples: usize,
}

/// A fila de blocos da gravação em curso. Armada no começo de cada gravação; os blocos chegam
/// pela thread da captura.
#[derive(Default)]
pub struct BlockQueue {
    state: Mutex<State>,
    /// A thread de uma gravação cancelada, que pode estar com o motor.
    lingering: Mutex<Option<JoinHandle<()>>>,
}

fn lock<T>(m: &Mutex<T>) -> MutexGuard<'_, T> {
    m.lock().unwrap_or_else(|poisoned| poisoned.into_inner())
}

impl BlockQueue {
    /// Começa uma gravação: com `Some`, os blocos dela são transcritos (e formatados) assim;
    /// com `None`, são ignorados. O que sobrou da gravação anterior é descartado.
    pub fn arm(&self, blocks: Option<Blocks>) {
        let previous = std::mem::replace(
            &mut *lock(&self.state),
            blocks.map_or(State::Off, State::Armed),
        );
        self.retire(previous);
    }

    /// Um bloco fechado. O primeiro sobe a thread de fundo.
    pub fn submit(&self, block: Vec<f32>) {
        let mut state = lock(&self.state);
        if let State::Armed(blocks) = &*state {
            let transcribe = Arc::clone(&blocks.transcribe);
            let format = blocks.format.clone();
            let lingering = lock(&self.lingering).take();
            *state = State::Running(Run::spawn(transcribe, format, lingering));
        }
        if let State::Running(run) = &mut *state {
            run.blocks += 1;
            run.samples += block.len();
            // Se a thread morreu, o `finish` devolve o erro.
            let _ = run.jobs.send((block, false));
        }
    }

    /// Esc: descarta os blocos que ainda não foram transcritos.
    pub fn cancel(&self) {
        let previous = std::mem::take(&mut *lock(&self.state));
        self.retire(previous);
    }

    /// O soltar, com a gravação inteira. Sem bloco fechado, devolve `None` depois de o motor
    /// ficar livre. Com blocos, transcreve o resto (`recording` depois deles) como o último
    /// pedaço e devolve os pedaços na ordem.
    pub fn finish(&self, recording: &[f32]) -> Option<Finished> {
        let state = std::mem::take(&mut *lock(&self.state));
        let State::Running(mut run) = state else {
            self.settle();
            return None;
        };
        let rest = recording.get(run.samples..).unwrap_or_default();
        if !rest.is_empty() {
            run.blocks += 1;
            let _ = run.jobs.send((rest.to_vec(), true));
        }
        let Run {
            jobs,
            results,
            worker,
            blocks,
            ..
        } = run;
        drop(jobs);
        let mut parts = Vec::with_capacity(blocks);
        let mut failure = None;
        for _ in 0..blocks {
            match results.recv() {
                Ok((Ok(raw), formatting)) => parts.push(Part { raw, formatting }),
                Ok((Err(e), _)) => {
                    failure.get_or_insert(e);
                }
                Err(_) => {
                    failure.get_or_insert(anyhow!("the block transcription thread stopped"));
                    break;
                }
            }
        }
        let _ = worker.join();
        let parts = match failure {
            Some(e) => Err(e),
            None => Ok(parts),
        };
        Some(Finished { parts, blocks })
    }

    fn retire(&self, state: State) {
        if let State::Running(run) = state {
            // Sem o receptor dos resultados, a thread para depois do bloco em que está.
            drop(run.jobs);
            drop(run.results);
            // A thread de antes desta já foi esperada por ela ao subir.
            *lock(&self.lingering) = Some(run.worker);
        }
    }

    fn settle(&self) {
        if let Some(worker) = lock(&self.lingering).take() {
            let _ = worker.join();
        }
    }
}

impl Run {
    fn spawn(
        transcribe: Transcribe,
        format: Option<Format>,
        lingering: Option<JoinHandle<()>>,
    ) -> Self {
        let (jobs, job_rx) = mpsc::channel::<(Vec<f32>, bool)>();
        let (done_tx, results) = mpsc::channel();
        let worker = std::thread::spawn(move || {
            if let Some(previous) = lingering {
                let _ = previous.join();
            }
            for (block, rest) in job_rx {
                let text = transcribe(block);
                let formatting = match (&text, &format) {
                    (Ok(raw), Some(format)) if !rest && !raw.trim().is_empty() => {
                        let (format, raw) = (Arc::clone(format), raw.clone());
                        Some(std::thread::spawn(move || format(raw)))
                    }
                    _ => None,
                };
                if done_tx.send((text, formatting)).is_err() {
                    return;
                }
            }
        });
        Self {
            jobs,
            results,
            worker,
            blocks: 0,
            samples: 0,
        }
    }
}

/// Os textos dos blocos, aparados, com um espaço entre eles; bloco sem texto não deixa espaço.
fn join<'a>(texts: impl IntoIterator<Item = &'a str>) -> String {
    texts
        .into_iter()
        .map(str::trim)
        .filter(|t| !t.is_empty())
        .collect::<Vec<_>>()
        .join(" ")
}

/// O ditado longo formatado bloco a bloco.
pub struct Composed {
    /// O texto a colar; as respostas que ainda podem chegar ficam em `late`, não em `late_edit`.
    pub processed: ProcessedTranscription,
    /// Algum bloco tem texto do LLM e outro ficou com as regras porque o LLM falhou ou atrasou.
    pub partial: bool,
    pub late: Option<LateBlocks>,
}

/// Junta, na ordem, os blocos formatados de um ditado cujo texto do ASR é `raw`. A resposta
/// tardia de um bloco que já chegou entra no lugar das regras; a que não chegou fica em `late`.
pub fn compose(raw: &str, blocks: Vec<ProcessedTranscription>) -> Composed {
    let mut finals = Vec::with_capacity(blocks.len());
    let mut pending = Vec::new();
    let (mut llm, mut fell_back) = (0, 0);
    let mut fallback = None;
    for (i, block) in blocks.into_iter().enumerate() {
        let mut text = block.final_text;
        let mut from_llm = block.llm_produced;
        let mut block_fallback = block.fallback;
        if let Some(late) = block.late_edit {
            match late.try_get() {
                Some(Ok(answer)) => {
                    text = answer;
                    from_llm = true;
                    block_fallback = None;
                }
                Some(Err(_)) => {}
                None => pending.push((i, late)),
            }
        }
        llm += usize::from(from_llm);
        if let Some(f) = block_fallback {
            fell_back += 1;
            fallback.get_or_insert(f);
        }
        finals.push(text);
    }
    let final_text = join(finals.iter().map(String::as_str));
    Composed {
        processed: ProcessedTranscription {
            post_processed_text: (final_text != raw).then(|| final_text.clone()),
            final_text,
            post_process_prompt: None,
            llm_produced: llm > 0,
            fallback,
            late_edit: None,
        },
        partial: llm > 0 && fell_back > 0,
        late: (!pending.is_empty()).then_some(LateBlocks {
            finals,
            pending,
            fell_back,
        }),
    }
}

/// Os blocos de um ditado à espera da resposta tardia do LLM.
pub struct LateBlocks {
    finals: Vec<String>,
    pending: Vec<(usize, LateEdit)>,
    /// Blocos que ficaram com as regras por falha do LLM, contando os pendentes.
    fell_back: usize,
}

impl LateBlocks {
    /// Espera cada resposta até o prazo tardio dela. Com ao menos uma, devolve o texto todo com
    /// elas no lugar das regras e se algum bloco ainda ficou sem o LLM.
    pub fn wait(self) -> Option<(String, bool)> {
        let LateBlocks {
            mut finals,
            pending,
            fell_back,
        } = self;
        let mut arrived = 0;
        for (i, late) in pending {
            match late.wait() {
                Ok(text) => {
                    finals[i] = text;
                    arrived += 1;
                }
                Err(e) => log::debug!("late LLM answer of a block not applied: {e}"),
            }
        }
        (arrived > 0).then(|| (join(finals.iter().map(String::as_str)), fell_back > arrived))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use fala_postproc::Fallback;
    use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
    use std::time::Duration;

    /// "Palavras" de teste: cada amostra é o código de uma palavra, e o texto de um bloco são os
    /// códigos dele. Um bloco de amostras vazias vira texto vazio.
    fn words(block: &[f32]) -> String {
        block
            .iter()
            .map(|c| format!("w{c}"))
            .collect::<Vec<_>>()
            .join(" ")
    }

    fn counting(calls: &Arc<AtomicUsize>) -> Transcribe {
        let calls = Arc::clone(calls);
        Arc::new(move |block: Vec<f32>| {
            calls.fetch_add(1, Ordering::SeqCst);
            Ok(words(&block))
        })
    }

    fn armed(transcribe: Transcribe) -> Option<Blocks> {
        Some(Blocks {
            transcribe,
            format: None,
        })
    }

    fn processed(text: &str, llm: bool, fallback: Option<Fallback>) -> ProcessedTranscription {
        ProcessedTranscription {
            final_text: text.to_owned(),
            post_processed_text: None,
            post_process_prompt: None,
            llm_produced: llm,
            fallback,
            late_edit: None,
        }
    }

    #[test]
    fn closed_blocks_are_formatted_while_recording_and_the_rest_is_not() {
        let (seen_tx, seen) = mpsc::channel();
        let seen_tx = Mutex::new(seen_tx);
        let queue = BlockQueue::default();
        queue.arm(Some(Blocks {
            transcribe: Arc::new(|block: Vec<f32>| Ok(words(&block))),
            format: Some(Arc::new(move |raw: String| {
                let _ = lock(&seen_tx).send(raw.clone());
                processed(&raw.to_uppercase(), true, None)
            })),
        }));
        let all = recording(6);
        queue.submit(all[..2].to_vec());
        assert_eq!(
            seen.recv_timeout(Duration::from_secs(5)).unwrap(),
            "w0 w1",
            "the first block is formatted before the release"
        );
        queue.submit(all[2..4].to_vec());
        let done = queue.finish(&all).unwrap();
        let parts = done.parts.unwrap();
        let raws: Vec<&str> = parts.iter().map(|p| p.raw.as_str()).collect();
        assert_eq!(raws, ["w0 w1", "w2 w3", "w4 w5"]);
        let formatted: Vec<String> = parts
            .into_iter()
            .filter_map(|p| p.formatting)
            .map(|f| f.join().unwrap().final_text)
            .collect();
        assert_eq!(formatted, ["W0 W1", "W2 W3"], "the rest is the caller's");
    }

    #[test]
    fn compose_joins_the_llm_blocks() {
        let c = compose(
            "um dois tres",
            vec![
                processed("Um.", true, None),
                processed("  ", true, None),
                processed("Dois três.", true, None),
            ],
        );
        assert_eq!(c.processed.final_text, "Um. Dois três.");
        assert_eq!(
            c.processed.post_processed_text.as_deref(),
            Some("Um. Dois três.")
        );
        assert!(c.processed.llm_produced);
        assert!(!c.partial);
        assert!(c.late.is_none());
        assert_eq!(c.processed.fallback, None);
    }

    #[test]
    fn compose_marks_rules_in_a_failed_block_as_partial() {
        let c = compose(
            "um dois",
            vec![
                processed("Um.", true, None),
                processed("Dois", false, Some(Fallback::Http(500))),
            ],
        );
        assert_eq!(c.processed.final_text, "Um. Dois");
        assert!(c.processed.llm_produced);
        assert!(c.partial);
        assert_eq!(c.processed.fallback, Some(Fallback::Http(500)));
        assert!(c.late.is_none(), "an http error never answers late");
    }

    #[test]
    fn compose_without_any_llm_or_with_short_blocks_is_not_partial() {
        let rules = compose(
            "um dois",
            vec![
                processed("Um", false, Some(Fallback::Network)),
                processed("Dois", false, Some(Fallback::Network)),
            ],
        );
        assert!(!rules.processed.llm_produced);
        assert!(!rules.partial, "no block has LLM text");

        // A block of 15 words or fewer never asks the LLM: rules there are the normal case.
        let short = compose(
            "um dois",
            vec![processed("Um.", true, None), processed("Dois", false, None)],
        );
        assert!(!short.partial);
        assert_eq!(short.processed.final_text, "Um. Dois");
        let same = compose("Um. Dois", vec![processed("Um. Dois", false, None)]);
        assert_eq!(same.processed.post_processed_text, None);
    }

    fn recording(n: usize) -> Vec<f32> {
        (0..n).map(|i| i as f32).collect()
    }

    #[test]
    fn blocks_and_rest_join_without_repeating_or_losing_words() {
        let calls = Arc::new(AtomicUsize::new(0));
        let queue = BlockQueue::default();
        queue.arm(armed(counting(&calls)));
        let all = recording(10);
        queue.submit(all[..4].to_vec());
        queue.submit(all[4..7].to_vec());
        let done = queue.finish(&all).expect("blocks closed");
        assert_eq!(done.raw().unwrap(), words(&all));
        assert_eq!(done.blocks, 3);
        assert_eq!(calls.load(Ordering::SeqCst), 3);
    }

    #[test]
    fn empty_text_and_an_empty_rest_leave_no_gap() {
        let queue = BlockQueue::default();
        queue.arm(armed(Arc::new(|block: Vec<f32>| {
            Ok(if block[0] == 0.0 {
                "  ".to_owned()
            } else {
                format!(" {} ", words(&block))
            })
        })));
        let all = recording(6);
        queue.submit(all[..2].to_vec());
        queue.submit(all[2..4].to_vec());
        queue.submit(all[4..].to_vec());
        let done = queue.finish(&all).unwrap();
        assert_eq!(done.raw().unwrap(), "w2 w3 w4 w5");
        assert_eq!(done.blocks, 3, "nothing left after the last block");
    }

    #[test]
    fn without_a_closed_block_the_old_path_runs() {
        let calls = Arc::new(AtomicUsize::new(0));
        let queue = BlockQueue::default();
        queue.arm(armed(counting(&calls)));
        assert!(queue.finish(&recording(320_000)).is_none());
        assert_eq!(calls.load(Ordering::SeqCst), 0);
    }

    #[test]
    fn an_unarmed_queue_ignores_blocks() {
        let calls = Arc::new(AtomicUsize::new(0));
        let queue = BlockQueue::default();
        queue.submit(recording(4));
        assert!(queue.finish(&recording(8)).is_none());

        queue.arm(armed(counting(&calls)));
        queue.arm(None);
        queue.submit(recording(4));
        assert!(queue.finish(&recording(8)).is_none());
        assert_eq!(calls.load(Ordering::SeqCst), 0);
    }

    #[test]
    fn a_failed_block_fails_the_dictation() {
        let queue = BlockQueue::default();
        queue.arm(armed(Arc::new(|block: Vec<f32>| {
            if block[0] == 2.0 {
                Err(anyhow!("engine failed"))
            } else {
                Ok(words(&block))
            }
        })));
        let all = recording(6);
        queue.submit(all[..2].to_vec());
        queue.submit(all[2..4].to_vec());
        let done = queue.finish(&all).unwrap();
        assert_eq!(done.raw().unwrap_err().to_string(), "engine failed");
        assert_eq!(done.blocks, 3);
    }

    #[test]
    fn cancel_drops_the_queue_and_frees_the_engine() {
        let calls = Arc::new(AtomicUsize::new(0));
        let in_engine = Arc::new(AtomicBool::new(false));
        let (entered_tx, entered) = mpsc::channel();
        let slow: Transcribe = {
            let calls = Arc::clone(&calls);
            let in_engine = Arc::clone(&in_engine);
            let entered_tx = Mutex::new(entered_tx);
            Arc::new(move |block: Vec<f32>| {
                calls.fetch_add(1, Ordering::SeqCst);
                in_engine.store(true, Ordering::SeqCst);
                let _ = lock(&entered_tx).send(());
                std::thread::sleep(Duration::from_millis(200));
                in_engine.store(false, Ordering::SeqCst);
                Ok(words(&block))
            })
        };
        let queue = BlockQueue::default();
        queue.arm(armed(slow));
        let all = recording(9);
        queue.submit(all[..3].to_vec());
        queue.submit(all[3..6].to_vec());
        queue.submit(all[6..].to_vec());
        entered.recv().unwrap();
        queue.cancel();

        assert!(
            queue.finish(&all).is_none(),
            "the cancelled dictation is gone"
        );
        assert!(
            !in_engine.load(Ordering::SeqCst),
            "finish waits for the block in the engine"
        );
        assert_eq!(
            calls.load(Ordering::SeqCst),
            1,
            "the pending blocks are dropped"
        );

        // The next dictation starts clean.
        let fresh = Arc::new(AtomicUsize::new(0));
        queue.arm(armed(counting(&fresh)));
        queue.submit(all[..5].to_vec());
        let done = queue.finish(&all).unwrap();
        assert_eq!(done.raw().unwrap(), words(&all));
        assert_eq!(fresh.load(Ordering::SeqCst), 2);
    }

    #[test]
    fn a_new_run_waits_for_the_cancelled_one() {
        let in_engine = Arc::new(AtomicBool::new(false));
        let overlapped = Arc::new(AtomicBool::new(false));
        let (entered_tx, entered) = mpsc::channel();
        let make = |signal: bool| -> Transcribe {
            let in_engine = Arc::clone(&in_engine);
            let overlapped = Arc::clone(&overlapped);
            let entered_tx = Mutex::new(entered_tx.clone());
            Arc::new(move |block: Vec<f32>| {
                if in_engine.swap(true, Ordering::SeqCst) {
                    overlapped.store(true, Ordering::SeqCst);
                }
                if signal {
                    let _ = lock(&entered_tx).send(());
                }
                std::thread::sleep(Duration::from_millis(100));
                in_engine.store(false, Ordering::SeqCst);
                Ok(words(&block))
            })
        };
        let queue = BlockQueue::default();
        queue.arm(armed(make(true)));
        queue.submit(recording(2));
        entered.recv().unwrap();
        queue.arm(armed(make(false)));
        queue.submit(recording(2));
        let done = queue.finish(&recording(2)).unwrap();
        assert_eq!(done.raw().unwrap(), "w0 w1");
        assert!(!overlapped.load(Ordering::SeqCst));
    }
}
