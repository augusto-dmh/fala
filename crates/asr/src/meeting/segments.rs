//! Das palavras do provedor aos segmentos "Eu / Pessoa N" (door 4 do plano `meeting-asr`).

use std::collections::HashMap;
use std::num::NonZeroU32;

use fala_meeting::SessionMode;
use serde::{Deserialize, Serialize};

/// De que lado da call veio o trecho. A ordem (`Mic` antes de `System`) desempata o merge.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Channel {
    Mic,
    System,
}

/// Quem fala: `me` (o canal do mic fora do modo presencial) ou uma pessoa numerada a partir
/// de 1. Serializa como `"me"` ou `{"person": N}`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Speaker {
    Me,
    Person(NonZeroU32),
}

/// Um trecho transcrito, com tempos em milissegundos desde o início do canal.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Segment {
    pub channel: Channel,
    pub speaker: Speaker,
    pub t0_ms: u64,
    pub t1_ms: u64,
    pub text: String,
}

/// Um trecho de um canal antes da numeração: o falante ainda é o rótulo do backend
/// (`speaker_id` da Scribe; `None` quando não há diarização).
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ChannelSegment {
    pub(crate) label: Option<String>,
    pub(crate) t0_ms: u64,
    pub(crate) t1_ms: u64,
    pub(crate) text: String,
}

/// O que interessa da resposta da Scribe. `language_code` e `text` não entram nos segmentos.
#[derive(Debug, Deserialize)]
pub(crate) struct ScribeResponse {
    #[serde(default)]
    pub(crate) words: Vec<ScribeWord>,
}

/// Um item de `words[]`: `type` é `word`, `spacing` ou `audio_event`; tempos em segundos.
#[derive(Debug, Clone, Deserialize)]
pub(crate) struct ScribeWord {
    pub(crate) text: String,
    #[serde(rename = "type")]
    pub(crate) kind: String,
    #[serde(default)]
    pub(crate) start: Option<f64>,
    #[serde(default)]
    pub(crate) end: Option<f64>,
    #[serde(default)]
    pub(crate) speaker_id: Option<String>,
}

fn secs_to_ms(secs: f64) -> u64 {
    // `as` satura: negativo e NaN viram 0.
    (secs * 1000.0).round() as u64
}

/// Junta palavras consecutivas do mesmo falante num trecho, com `t0_ms` da primeira e `t1_ms`
/// da última. `spacing` só entra no texto entre palavras do mesmo trecho; os outros tipos
/// (`audio_event`) não entram no texto nem criam falante.
pub(crate) fn group_words(words: &[ScribeWord]) -> Vec<ChannelSegment> {
    let mut out: Vec<ChannelSegment> = Vec::new();
    let mut spacing = String::new();
    for word in words {
        match word.kind.as_str() {
            "word" => {
                let (Some(start), Some(end)) = (word.start, word.end) else {
                    continue;
                };
                let (t0_ms, t1_ms) = (secs_to_ms(start), secs_to_ms(end));
                match out.last_mut() {
                    Some(current) if current.label == word.speaker_id => {
                        current.text.push_str(&spacing);
                        current.text.push_str(&word.text);
                        current.t1_ms = current.t1_ms.max(t1_ms);
                    }
                    _ => out.push(ChannelSegment {
                        label: word.speaker_id.clone(),
                        t0_ms,
                        t1_ms,
                        text: word.text.clone(),
                    }),
                }
                spacing.clear();
            }
            "spacing" => spacing.clone_from(&word.text),
            _ => {}
        }
    }
    for segment in &mut out {
        segment.text = segment.text.trim().to_string();
    }
    out
}

/// Numera os falantes e junta os canais. Fora do modo presencial, o mic é `me`; no presencial,
/// os rótulos do mic viram `person 1..k` e os do sistema continuam em `k+1..`. Cada rótulo
/// recebe seu número na ordem de primeira aparição.
pub(crate) fn assign_speakers(
    mode: SessionMode,
    mic: Vec<ChannelSegment>,
    system: Vec<ChannelSegment>,
) -> Vec<Segment> {
    let mut numbering = Numbering::default();
    let mic = mic
        .into_iter()
        .map(|segment| {
            let speaker = if mode == SessionMode::InPerson {
                numbering.person(Channel::Mic, &segment.label)
            } else {
                Speaker::Me
            };
            segment.into_segment(Channel::Mic, speaker)
        })
        .collect();
    let system = system
        .into_iter()
        .map(|segment| {
            let speaker = numbering.person(Channel::System, &segment.label);
            segment.into_segment(Channel::System, speaker)
        })
        .collect();
    merge(mic, system)
}

/// Uma lista só, ordenada por `t0_ms`; em empate, o mic vem antes do sistema e cada canal
/// mantém a própria ordem.
pub(crate) fn merge(mic: Vec<Segment>, system: Vec<Segment>) -> Vec<Segment> {
    let mut all: Vec<Segment> = mic.into_iter().chain(system).collect();
    all.sort_by_key(|segment| (segment.t0_ms, segment.channel));
    all
}

impl ChannelSegment {
    fn into_segment(self, channel: Channel, speaker: Speaker) -> Segment {
        Segment {
            channel,
            speaker,
            t0_ms: self.t0_ms,
            t1_ms: self.t1_ms,
            text: self.text,
        }
    }
}

/// Números de pessoa por (canal, rótulo), atribuídos em sequência a partir de 1.
#[derive(Default)]
struct Numbering {
    seen: HashMap<(Channel, Option<String>), NonZeroU32>,
}

impl Numbering {
    fn person(&mut self, channel: Channel, label: &Option<String>) -> Speaker {
        let next = NonZeroU32::MIN.saturating_add(self.seen.len() as u32);
        Speaker::Person(*self.seen.entry((channel, label.clone())).or_insert(next))
    }
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used, clippy::expect_used)]

    use super::*;

    fn word(text: &str, start: f64, end: f64, speaker: &str) -> ScribeWord {
        ScribeWord {
            text: text.to_string(),
            kind: "word".to_string(),
            start: Some(start),
            end: Some(end),
            speaker_id: Some(speaker.to_string()),
        }
    }

    fn item(kind: &str, text: &str, start: f64, end: f64, speaker: Option<&str>) -> ScribeWord {
        ScribeWord {
            text: text.to_string(),
            kind: kind.to_string(),
            start: Some(start),
            end: Some(end),
            speaker_id: speaker.map(str::to_string),
        }
    }

    fn person(n: u32) -> Speaker {
        Speaker::Person(NonZeroU32::new(n).unwrap())
    }

    fn raw(label: &str, t0_ms: u64, text: &str) -> ChannelSegment {
        ChannelSegment {
            label: Some(label.to_string()),
            t0_ms,
            t1_ms: t0_ms + 500,
            text: text.to_string(),
        }
    }

    #[test]
    fn groups_consecutive_words_by_speaker() {
        let json = r#"{
            "language_code": "por",
            "text": "Oi tudo [risos] bem",
            "words": [
                {"text": "Oi", "start": 0.12, "end": 0.40, "type": "word", "speaker_id": "speaker_0"},
                {"text": " ", "start": 0.40, "end": 0.45, "type": "spacing", "speaker_id": "speaker_0"},
                {"text": "tudo", "start": 0.45, "end": 0.90, "type": "word", "speaker_id": "speaker_0"},
                {"text": " ", "start": 0.90, "end": 1.00, "type": "spacing", "speaker_id": "speaker_0"},
                {"text": "[risos]", "start": 1.00, "end": 1.50, "type": "audio_event", "speaker_id": "speaker_9"},
                {"text": "bem", "start": 1.60, "end": 2.05, "type": "word", "speaker_id": "speaker_1"}
            ]
        }"#;
        let response: ScribeResponse = serde_json::from_str(json).unwrap();
        let grouped = group_words(&response.words);
        assert_eq!(
            grouped,
            vec![
                ChannelSegment {
                    label: Some("speaker_0".to_string()),
                    t0_ms: 120,
                    t1_ms: 900,
                    text: "Oi tudo".to_string(),
                },
                ChannelSegment {
                    label: Some("speaker_1".to_string()),
                    t0_ms: 1_600,
                    t1_ms: 2_050,
                    text: "bem".to_string(),
                },
            ]
        );
        // O `audio_event` não vira falante: numerados, só há duas pessoas.
        let segments = assign_speakers(SessionMode::Meeting, Vec::new(), grouped);
        assert_eq!(
            segments.iter().map(|s| s.speaker).collect::<Vec<_>>(),
            [person(1), person(2)]
        );

        // Um `audio_event` sozinho, ou com `spacing`, não cria trecho nenhum.
        let only_event = [
            item("audio_event", "[música]", 0.0, 3.0, Some("speaker_4")),
            item("spacing", " ", 3.0, 3.1, Some("speaker_4")),
        ];
        assert!(group_words(&only_event).is_empty());
    }

    #[test]
    fn mic_is_me_system_numbered_by_first_appearance() {
        let mic = group_words(&[
            word("eu", 0.0, 0.3, "speaker_0"),
            word("acho", 0.4, 0.8, "speaker_1"),
        ]);
        let system = group_words(&[
            word("bom", 0.5, 0.9, "speaker_7"),
            word("dia", 1.0, 1.4, "speaker_2"),
            word("certo", 2.0, 2.4, "speaker_7"),
        ]);
        let segments = assign_speakers(SessionMode::Meeting, mic, system);
        let mic: Vec<_> = segments
            .iter()
            .filter(|s| s.channel == Channel::Mic)
            .collect();
        assert_eq!(mic.len(), 2);
        assert!(mic.iter().all(|s| s.speaker == Speaker::Me), "{mic:?}");
        let system: Vec<_> = segments
            .iter()
            .filter(|s| s.channel == Channel::System)
            .map(|s| (s.text.as_str(), s.speaker))
            .collect();
        assert_eq!(
            system,
            [("bom", person(1)), ("dia", person(2)), ("certo", person(1))]
        );

        // `system_only`: só o sistema tem fala, numerada do mesmo jeito.
        let system = vec![raw("speaker_3", 0, "a"), raw("speaker_1", 900, "b")];
        let segments = assign_speakers(SessionMode::SystemOnly, Vec::new(), system);
        assert_eq!(
            segments.iter().map(|s| s.speaker).collect::<Vec<_>>(),
            [person(1), person(2)]
        );
    }

    #[test]
    fn in_person_numbering_continues_across_channels() {
        let mic = vec![
            raw("speaker_1", 0, "a"),
            raw("speaker_0", 1_000, "b"),
            raw("speaker_1", 2_000, "c"),
        ];
        let system = vec![raw("speaker_0", 500, "d")];
        let segments = assign_speakers(SessionMode::InPerson, mic, system);
        let got: Vec<_> = segments
            .iter()
            .map(|s| (s.channel, s.text.as_str(), s.speaker))
            .collect();
        assert_eq!(
            got,
            [
                (Channel::Mic, "a", person(1)),
                (Channel::System, "d", person(3)),
                (Channel::Mic, "b", person(2)),
                (Channel::Mic, "c", person(1)),
            ]
        );
        assert!(segments.iter().all(|s| s.speaker != Speaker::Me));
    }

    fn at(channel: Channel, t0_ms: u64) -> Segment {
        Segment {
            channel,
            speaker: Speaker::Me,
            t0_ms,
            t1_ms: t0_ms + 100,
            text: String::new(),
        }
    }

    #[test]
    fn merge_orders_by_t0_mic_first_on_tie() {
        let mic = vec![
            at(Channel::Mic, 0),
            at(Channel::Mic, 5_000),
            at(Channel::Mic, 9_000),
        ];
        let system = vec![at(Channel::System, 2_000), at(Channel::System, 5_000)];
        let merged = merge(mic, system);
        let got: Vec<_> = merged.iter().map(|s| (s.t0_ms, s.channel)).collect();
        assert_eq!(
            got,
            [
                (0, Channel::Mic),
                (2_000, Channel::System),
                (5_000, Channel::Mic),
                (5_000, Channel::System),
                (9_000, Channel::Mic),
            ]
        );
        // O empate não depende da ordem dos argumentos internos: sistema primeiro na entrada.
        let merged = merge(
            vec![at(Channel::Mic, 5_000)],
            vec![at(Channel::System, 5_000)],
        );
        assert_eq!(merged[0].channel, Channel::Mic);
    }

    #[test]
    fn segment_serialized_form() {
        let mic = Segment {
            channel: Channel::Mic,
            speaker: Speaker::Me,
            t0_ms: 0,
            t1_ms: 1_200,
            text: "oi".to_string(),
        };
        let json = serde_json::to_string(&mic).unwrap();
        assert_eq!(
            json,
            r#"{"channel":"mic","speaker":"me","t0_ms":0,"t1_ms":1200,"text":"oi"}"#
        );
        assert_eq!(serde_json::from_str::<Segment>(&json).unwrap(), mic);

        let system = Segment {
            channel: Channel::System,
            speaker: person(2),
            t0_ms: 1_500,
            t1_ms: 2_000,
            text: "tudo bem?".to_string(),
        };
        let json = serde_json::to_string(&system).unwrap();
        assert_eq!(
            json,
            r#"{"channel":"system","speaker":{"person":2},"t0_ms":1500,"t1_ms":2000,"text":"tudo bem?"}"#
        );
        assert_eq!(serde_json::from_str::<Segment>(&json).unwrap(), system);

        // `N` ≥ 1 pela forma: `person 0` não desserializa.
        let zero = r#"{"channel":"system","speaker":{"person":0},"t0_ms":0,"t1_ms":1,"text":""}"#;
        assert!(serde_json::from_str::<Segment>(zero).is_err());
    }
}
