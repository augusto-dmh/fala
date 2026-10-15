//! Quando repetir uma transcrição de reunião que falhou na nuvem (ADR-0005: nada se perde se a
//! transcrição falhar). Puro: quem chama dá o relógio e roda as tentativas; aqui só se decide
//! quando, e por quantas vezes.

use std::collections::HashMap;
use std::hash::Hash;
use std::time::{Duration, Instant};

use crate::AsrError;

/// Que tipo de falha a nuvem devolveu, para decidir a próxima tentativa.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Failure {
    /// Sem resposta HTTP (sem rede, DNS, conexão caída): quase sempre o envio falha antes de
    /// subir o áudio, então tentar de novo é barato. Uma conexão que cai depois do envio parece
    /// igual, por isso também há limite ([`OFFLINE_MAX_FAILURES`]).
    Offline,
    /// O provedor respondeu `429` ou `5xx`: repetir pode dar certo, mas cada tentativa sobe o
    /// áudio inteiro.
    Busy,
    /// Repetir a mesma requisição não muda nada (chave recusada, `4xx`, resposta inválida).
    Final,
}

impl Failure {
    pub fn of(error: &AsrError) -> Self {
        match error {
            AsrError::Network {
                retriable: true,
                status: None,
                ..
            } => Failure::Offline,
            AsrError::Network {
                retriable: true,
                status: Some(_),
                ..
            } => Failure::Busy,
            _ => Failure::Final,
        }
    }
}

/// Esperas depois de cada falha sem rede; da última em diante, a mesma espera até o limite.
pub const OFFLINE_DELAYS: [Duration; 4] = [
    Duration::from_secs(10),
    Duration::from_secs(30),
    Duration::from_secs(60),
    Duration::from_secs(5 * 60),
];
/// Falhas seguidas sem rede até desistir (uns 20 min com as esperas acima).
pub const OFFLINE_MAX_FAILURES: u32 = 8;
/// Esperas depois de cada `429`/`5xx`; acabadas, a sessão desiste.
pub const BUSY_DELAYS: [Duration; 3] = [
    Duration::from_secs(60),
    Duration::from_secs(5 * 60),
    Duration::from_secs(15 * 60),
];

/// O que fazer depois de uma falha.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Next {
    RetryAt(Instant),
    GiveUp,
}

/// Uma sessão na fila.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Waiting {
    /// Falhas seguidas da nuvem.
    pub failures: u32,
    /// A última falha foi sem rede (e não do provedor).
    pub offline: bool,
    /// `None` enquanto a tentativa roda.
    due: Option<Instant>,
}

/// As sessões cuja transcrição espera a próxima tentativa.
#[derive(Debug)]
pub struct RetryQueue<K> {
    waiting: HashMap<K, Waiting>,
}

impl<K> Default for RetryQueue<K> {
    fn default() -> Self {
        Self {
            waiting: HashMap::new(),
        }
    }
}

impl<K: Copy + Eq + Hash> RetryQueue<K> {
    pub fn new() -> Self {
        Self::default()
    }

    /// Anota a falha de `key` e marca a próxima tentativa; [`Next::GiveUp`] tira a chave da fila.
    pub fn failed(&mut self, key: K, failure: Failure, now: Instant) -> Next {
        let failures = self.waiting.get(&key).map_or(0, |w| w.failures) + 1;
        let index = usize::try_from(failures - 1).unwrap_or(usize::MAX);
        let delay = match failure {
            Failure::Offline if failures < OFFLINE_MAX_FAILURES => {
                Some(OFFLINE_DELAYS[index.min(OFFLINE_DELAYS.len() - 1)])
            }
            Failure::Offline => None,
            Failure::Busy => BUSY_DELAYS.get(index).copied(),
            Failure::Final => None,
        };
        let Some(delay) = delay else {
            self.waiting.remove(&key);
            return Next::GiveUp;
        };
        let at = now + delay;
        self.waiting.insert(
            key,
            Waiting {
                failures,
                offline: failure == Failure::Offline,
                due: Some(at),
            },
        );
        Next::RetryAt(at)
    }

    /// Tenta de novo em `at` sem contar falha (outra transcrição ocupava a vez).
    pub fn later(&mut self, key: K, at: Instant) {
        self.waiting
            .entry(key)
            .and_modify(|w| w.due = Some(at))
            .or_insert(Waiting {
                failures: 0,
                offline: false,
                due: Some(at),
            });
    }

    /// Tira `key` da fila (a transcrição chegou, foi cancelada ou não tem mais como rodar).
    pub fn forget(&mut self, key: K) -> Option<Waiting> {
        self.waiting.remove(&key)
    }

    pub fn get(&self, key: K) -> Option<Waiting> {
        self.waiting.get(&key).copied()
    }

    /// As chaves cuja hora chegou, já marcadas como rodando: até `failed`, `later` ou `forget`,
    /// não voltam aqui nem em [`next_due`](Self::next_due).
    pub fn take_due(&mut self, now: Instant) -> Vec<K> {
        let mut due = Vec::new();
        for (key, waiting) in &mut self.waiting {
            if waiting.due.is_some_and(|at| at <= now) {
                waiting.due = None;
                due.push(*key);
            }
        }
        due
    }

    /// A próxima tentativa marcada, se houver.
    pub fn next_due(&self) -> Option<Instant> {
        self.waiting.values().filter_map(|w| w.due).min()
    }

    /// O PC acordou: toda tentativa marcada para depois de `at` passa para
    /// `at`.
    pub fn wake(&mut self, at: Instant) {
        for waiting in self.waiting.values_mut() {
            if let Some(due) = &mut waiting.due {
                *due = (*due).min(at);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used, clippy::expect_used)]

    use super::*;

    fn network(status: Option<u16>, retriable: bool) -> AsrError {
        AsrError::Network {
            retriable,
            status,
            reason: String::new(),
        }
    }

    #[test]
    fn failures_are_classified_by_status() {
        assert_eq!(Failure::of(&network(None, true)), Failure::Offline);
        assert_eq!(Failure::of(&network(Some(429), true)), Failure::Busy);
        assert_eq!(Failure::of(&network(Some(503), true)), Failure::Busy);
        assert_eq!(Failure::of(&network(Some(401), false)), Failure::Final);
        assert_eq!(
            Failure::of(&AsrError::InvalidResponse(String::new())),
            Failure::Final
        );
        assert_eq!(Failure::of(&AsrError::Cancelled), Failure::Final);
    }

    #[test]
    fn offline_backs_off_then_gives_up() {
        let t0 = Instant::now();
        let mut queue = RetryQueue::new();
        let mut delays = Vec::new();
        while let Next::RetryAt(at) = queue.failed(1, Failure::Offline, t0) {
            delays.push((at - t0).as_secs());
            let waiting = queue.get(1).unwrap();
            assert!(waiting.offline);
        }
        assert_eq!(delays, [10, 30, 60, 300, 300, 300, 300]);
        assert_eq!(queue.get(1), None, "a oitava falha desiste");
    }

    #[test]
    fn busy_gives_up_after_three_retries() {
        let t0 = Instant::now();
        let mut queue = RetryQueue::new();
        let mut delays = Vec::new();
        while let Next::RetryAt(at) = queue.failed("s", Failure::Busy, t0) {
            delays.push((at - t0).as_secs());
        }
        assert_eq!(delays, [60, 300, 900]);
        assert_eq!(queue.get("s"), None, "desistir tira da fila");

        // Final desiste na hora; uma falha do provedor depois de uma sem rede segue contando.
        assert_eq!(queue.failed("f", Failure::Final, t0), Next::GiveUp);
        queue.failed("m", Failure::Offline, t0);
        assert_eq!(
            queue.failed("m", Failure::Busy, t0),
            Next::RetryAt(t0 + BUSY_DELAYS[1])
        );
        assert!(!queue.get("m").unwrap().offline);
    }

    #[test]
    fn due_keys_run_once_until_rescheduled() {
        let t0 = Instant::now();
        let mut queue = RetryQueue::new();
        queue.failed(1, Failure::Offline, t0); // +10 s
        queue.failed(2, Failure::Busy, t0); // +60 s
        assert_eq!(queue.next_due(), Some(t0 + Duration::from_secs(10)));
        assert!(queue.take_due(t0).is_empty());

        let at = t0 + Duration::from_secs(20);
        assert_eq!(queue.take_due(at), [1]);
        assert!(queue.take_due(at).is_empty(), "rodando não volta");
        assert_eq!(queue.next_due(), Some(t0 + Duration::from_secs(60)));

        // Ocupado: volta sem contar falha.
        queue.later(1, at + Duration::from_secs(30));
        assert_eq!(queue.get(1).unwrap().failures, 1);
        assert_eq!(queue.next_due(), Some(at + Duration::from_secs(30)));
        assert!(queue.forget(1).is_some());
        assert_eq!(queue.next_due(), Some(t0 + Duration::from_secs(60)));
    }

    #[test]
    fn waking_brings_every_retry_forward() {
        let t0 = Instant::now();
        let mut queue = RetryQueue::new();
        queue.failed(1, Failure::Busy, t0); // +60 s
        queue.failed(2, Failure::Offline, t0); // +10 s
        queue.take_due(t0 + Duration::from_secs(10)); // 2 está rodando
        let woke = t0 + Duration::from_secs(5);
        queue.wake(woke);
        assert_eq!(queue.next_due(), Some(woke));
        assert_eq!(queue.take_due(woke), [1], "quem roda não é remarcado");
    }
}
