//! Ditado longo: os blocos que o `DictationCapture` fecha durante a gravação são transcritos numa
//! thread de fundo, na ordem, e ao soltar só falta o resto. Sem bloco fechado, o ditado segue o
//! caminho de sempre (`TranscriptionManager::transcribe` no áudio inteiro).
//!
//! O motor do ASR atende uma chamada por vez (`transcribe` o tira do mutex), então a fila tem uma
//! thread só, o resto vai pela mesma fila, e depois de um cancelamento quem usa o motor de novo
//! espera o bloco que estava nele terminar.

use std::sync::{mpsc, Arc, Mutex, MutexGuard};
use std::thread::JoinHandle;

use anyhow::anyhow;

/// Transcreve um bloco a 16 kHz mono.
pub type Transcribe = Arc<dyn Fn(Vec<f32>) -> anyhow::Result<String> + Send + Sync>;

/// O texto de um ditado longo e em quantos pedaços o ASR rodou.
pub struct Finished {
    pub text: anyhow::Result<String>,
    pub blocks: usize,
}

#[derive(Default)]
enum State {
    /// Esta gravação não usa blocos.
    #[default]
    Off,
    /// Usa, e nenhum bloco fechou ainda.
    Armed(Transcribe),
    Running(Run),
}

struct Run {
    jobs: mpsc::Sender<Vec<f32>>,
    results: mpsc::Receiver<anyhow::Result<String>>,
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
    /// Começa uma gravação: com `Some`, os blocos dela vão para esse transcritor; com `None`,
    /// são ignorados. O que sobrou da gravação anterior é descartado.
    pub fn arm(&self, transcribe: Option<Transcribe>) {
        let previous = std::mem::replace(
            &mut *lock(&self.state),
            transcribe.map_or(State::Off, State::Armed),
        );
        self.retire(previous);
    }

    /// Um bloco fechado. O primeiro sobe a thread de fundo.
    pub fn submit(&self, block: Vec<f32>) {
        let mut state = lock(&self.state);
        if let State::Armed(transcribe) = &*state {
            let transcribe = Arc::clone(transcribe);
            let lingering = lock(&self.lingering).take();
            *state = State::Running(Run::spawn(transcribe, lingering));
        }
        if let State::Running(run) = &mut *state {
            run.blocks += 1;
            run.samples += block.len();
            // Se a thread morreu, o `finish` devolve o erro.
            let _ = run.jobs.send(block);
        }
    }

    /// Esc: descarta os blocos que ainda não foram transcritos.
    pub fn cancel(&self) {
        let previous = std::mem::take(&mut *lock(&self.state));
        self.retire(previous);
    }

    /// O soltar, com a gravação inteira. Sem bloco fechado, devolve `None` depois de o motor
    /// ficar livre. Com blocos, transcreve o resto (`recording` depois deles) como o último bloco
    /// e junta os textos na ordem.
    pub fn finish(&self, recording: &[f32]) -> Option<Finished> {
        let state = std::mem::take(&mut *lock(&self.state));
        let State::Running(mut run) = state else {
            self.settle();
            return None;
        };
        let rest = recording.get(run.samples..).unwrap_or_default();
        if !rest.is_empty() {
            run.blocks += 1;
            let _ = run.jobs.send(rest.to_vec());
        }
        let Run {
            jobs,
            results,
            worker,
            blocks,
            ..
        } = run;
        drop(jobs);
        let mut texts = Vec::with_capacity(blocks);
        let mut failure = None;
        for _ in 0..blocks {
            match results.recv() {
                Ok(Ok(text)) => texts.push(text),
                Ok(Err(e)) => {
                    failure.get_or_insert(e);
                }
                Err(_) => {
                    failure.get_or_insert(anyhow!("the block transcription thread stopped"));
                    break;
                }
            }
        }
        let _ = worker.join();
        let text = match failure {
            Some(e) => Err(e),
            None => Ok(join(&texts)),
        };
        Some(Finished { text, blocks })
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
    fn spawn(transcribe: Transcribe, lingering: Option<JoinHandle<()>>) -> Self {
        let (jobs, job_rx) = mpsc::channel::<Vec<f32>>();
        let (done_tx, results) = mpsc::channel();
        let worker = std::thread::spawn(move || {
            if let Some(previous) = lingering {
                let _ = previous.join();
            }
            for block in job_rx {
                if done_tx.send(transcribe(block)).is_err() {
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
fn join(texts: &[String]) -> String {
    texts
        .iter()
        .map(|t| t.trim())
        .filter(|t| !t.is_empty())
        .collect::<Vec<_>>()
        .join(" ")
}

#[cfg(test)]
mod tests {
    use super::*;
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

    fn recording(n: usize) -> Vec<f32> {
        (0..n).map(|i| i as f32).collect()
    }

    #[test]
    fn blocks_and_rest_join_without_repeating_or_losing_words() {
        let calls = Arc::new(AtomicUsize::new(0));
        let queue = BlockQueue::default();
        queue.arm(Some(counting(&calls)));
        let all = recording(10);
        queue.submit(all[..4].to_vec());
        queue.submit(all[4..7].to_vec());
        let done = queue.finish(&all).expect("blocks closed");
        assert_eq!(done.text.unwrap(), words(&all));
        assert_eq!(done.blocks, 3);
        assert_eq!(calls.load(Ordering::SeqCst), 3);
    }

    #[test]
    fn empty_text_and_an_empty_rest_leave_no_gap() {
        let queue = BlockQueue::default();
        queue.arm(Some(Arc::new(|block: Vec<f32>| {
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
        assert_eq!(done.text.unwrap(), "w2 w3 w4 w5");
        assert_eq!(done.blocks, 3, "nothing left after the last block");
    }

    #[test]
    fn without_a_closed_block_the_old_path_runs() {
        let calls = Arc::new(AtomicUsize::new(0));
        let queue = BlockQueue::default();
        queue.arm(Some(counting(&calls)));
        assert!(queue.finish(&recording(320_000)).is_none());
        assert_eq!(calls.load(Ordering::SeqCst), 0);
    }

    #[test]
    fn an_unarmed_queue_ignores_blocks() {
        let calls = Arc::new(AtomicUsize::new(0));
        let queue = BlockQueue::default();
        queue.submit(recording(4));
        assert!(queue.finish(&recording(8)).is_none());

        queue.arm(Some(counting(&calls)));
        queue.arm(None);
        queue.submit(recording(4));
        assert!(queue.finish(&recording(8)).is_none());
        assert_eq!(calls.load(Ordering::SeqCst), 0);
    }

    #[test]
    fn a_failed_block_fails_the_dictation() {
        let queue = BlockQueue::default();
        queue.arm(Some(Arc::new(|block: Vec<f32>| {
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
        assert_eq!(done.text.unwrap_err().to_string(), "engine failed");
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
        queue.arm(Some(slow));
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
        queue.arm(Some(counting(&fresh)));
        queue.submit(all[..5].to_vec());
        let done = queue.finish(&all).unwrap();
        assert_eq!(done.text.unwrap(), words(&all));
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
        queue.arm(Some(make(true)));
        queue.submit(recording(2));
        entered.recv().unwrap();
        queue.arm(Some(make(false)));
        queue.submit(recording(2));
        let done = queue.finish(&recording(2)).unwrap();
        assert_eq!(done.text.unwrap(), "w0 w1");
        assert!(!overlapped.load(Ordering::SeqCst));
    }
}
