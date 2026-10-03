//! `fala-cli key`: guarda, consulta e apaga chaves de API no keyring do SO (ADR-0008).
//!
//! A chave só entra pelo stdin e nunca é escrita em stdout, stderr ou log.

use std::io::{BufRead, Write};

use clap::{Args, Subcommand};
use fala_secrets::{validate_provider, ApiKey, SecretError, SecretStore};

#[derive(Args)]
pub struct KeyArgs {
    #[command(subcommand)]
    action: KeyAction,
}

#[derive(Subcommand)]
enum KeyAction {
    /// Guarda a chave lida da primeira linha do stdin. Por pipe ela não aparece na tela:
    /// `printf '%s' "$CHAVE" | fala-cli key set gemini`; digitada no terminal, aparece.
    Set {
        /// Id do provedor (`gemini`, `openai`, ...): 1 a 32 caracteres entre a-z, 0-9 e _.
        provider: String,
    },
    /// Imprime `definida` ou `ausente`.
    Status {
        /// Id do provedor.
        provider: String,
    },
    /// Apaga a chave; também sai com 0 se ela já não existia.
    Delete {
        /// Id do provedor.
        provider: String,
    },
}

/// Roda o subcomando e devolve o código de saída: 0 ok, 1 keyring, 2 uso inválido.
pub fn run(
    args: KeyArgs,
    store: &dyn SecretStore,
    stdin: &mut dyn BufRead,
    stdout: &mut dyn Write,
    stderr: &mut dyn Write,
) -> u8 {
    match execute(args.action, store, stdin, stdout, stderr) {
        Ok(()) => 0,
        Err(error) => report(&error, stderr),
    }
}

fn execute(
    action: KeyAction,
    store: &dyn SecretStore,
    stdin: &mut dyn BufRead,
    stdout: &mut dyn Write,
    stderr: &mut dyn Write,
) -> Result<(), SecretError> {
    match action {
        KeyAction::Set { provider } => {
            validate_provider(&provider)?;
            let mut line = String::new();
            stdin
                .read_line(&mut line)
                .map_err(|_| SecretError::EmptyKey)?;
            let key = ApiKey::new(line.trim_end_matches(['\n', '\r']))?;
            store.set(&provider, &key)?;
            let _ = writeln!(stderr, "chave de {provider} guardada");
        }
        KeyAction::Status { provider } => {
            let state = match store.get(&provider)? {
                Some(_) => "definida",
                None => "ausente",
            };
            let _ = writeln!(stdout, "{state}");
        }
        KeyAction::Delete { provider } => {
            store.delete(&provider)?;
            let _ = writeln!(stderr, "chave de {provider} removida");
        }
    }
    Ok(())
}

/// Escreve o erro no stderr e devolve o código de saída correspondente.
pub fn report(error: &SecretError, stderr: &mut dyn Write) -> u8 {
    match error {
        SecretError::InvalidProvider => {
            let _ = writeln!(stderr, "erro: {error}");
            2
        }
        SecretError::EmptyKey => {
            let _ = writeln!(
                stderr,
                "erro: stdin vazio; passe a chave na primeira linha do stdin"
            );
            2
        }
        SecretError::Unavailable | SecretError::Store => {
            let _ = writeln!(stderr, "erro: keyring indisponível ({error})");
            1
        }
    }
}

#[cfg(test)]
pub(crate) mod tests {
    use super::*;
    use fala_secrets::MemoryStore;

    /// Um keyring que nunca responde.
    pub(crate) struct UnavailableStore;

    impl SecretStore for UnavailableStore {
        fn get(&self, _: &str) -> Result<Option<ApiKey>, SecretError> {
            Err(SecretError::Unavailable)
        }
        fn set(&self, _: &str, _: &ApiKey) -> Result<(), SecretError> {
            Err(SecretError::Unavailable)
        }
        fn delete(&self, _: &str) -> Result<(), SecretError> {
            Err(SecretError::Unavailable)
        }
    }

    /// Um keyring que responde, mas recusa toda operação.
    pub(crate) struct RefusingStore;

    impl SecretStore for RefusingStore {
        fn get(&self, _: &str) -> Result<Option<ApiKey>, SecretError> {
            Err(SecretError::Store)
        }
        fn set(&self, _: &str, _: &ApiKey) -> Result<(), SecretError> {
            Err(SecretError::Store)
        }
        fn delete(&self, _: &str) -> Result<(), SecretError> {
            Err(SecretError::Store)
        }
    }

    struct Outcome {
        code: u8,
        stdout: String,
        stderr: String,
    }

    fn key(store: &dyn SecretStore, argv: &[&str], stdin: &str) -> Outcome {
        key_bytes(store, argv, stdin.as_bytes())
    }

    fn key_bytes(store: &dyn SecretStore, argv: &[&str], stdin: &[u8]) -> Outcome {
        use clap::Parser;
        #[derive(Parser)]
        struct Wrapper {
            #[command(flatten)]
            args: KeyArgs,
        }
        let mut full = vec!["key"];
        full.extend_from_slice(argv);
        let args = Wrapper::try_parse_from(full).unwrap().args;
        let (mut out, mut err) = (Vec::new(), Vec::new());
        let code = run(args, store, &mut &stdin[..], &mut out, &mut err);
        Outcome {
            code,
            stdout: String::from_utf8(out).unwrap(),
            stderr: String::from_utf8(err).unwrap(),
        }
    }

    #[test]
    fn set_stores_and_never_echoes() {
        let store = MemoryStore::default();
        let out = key(&store, &["set", "gemini"], "k-sentinela\n");
        assert_eq!(out.code, 0);
        assert_eq!(
            store.get("gemini").unwrap(),
            Some(ApiKey::new("k-sentinela").unwrap())
        );
        assert!(!out.stdout.contains("k-sentinela"));
        assert!(!out.stderr.contains("k-sentinela"));
        assert!(out.stderr.contains("chave de gemini guardada"));
    }

    #[test]
    fn set_empty_stdin_exits_2() {
        for stdin in ["", "\n"] {
            let store = MemoryStore::default();
            let out = key(&store, &["set", "gemini"], stdin);
            assert_eq!(out.code, 2, "{stdin:?}");
            assert!(store.is_empty(), "{stdin:?}");
        }
    }

    #[test]
    fn status_delete_and_invalid_provider() {
        let store = MemoryStore::default();
        let out = key(&store, &["status", "gemini"], "");
        assert_eq!((out.code, out.stdout.as_str()), (0, "ausente\n"));

        assert_eq!(key(&store, &["set", "gemini"], "valor\n").code, 0);
        let out = key(&store, &["status", "gemini"], "");
        assert_eq!((out.code, out.stdout.as_str()), (0, "definida\n"));

        let out = key(&store, &["delete", "gemini"], "");
        assert_eq!(out.code, 0);
        assert!(out.stderr.contains("chave de gemini removida"));
        let out = key(&store, &["status", "gemini"], "");
        assert_eq!((out.code, out.stdout.as_str()), (0, "ausente\n"));
        assert_eq!(key(&store, &["delete", "gemini"], "").code, 0);

        let out = key(&store, &["set", "Gemini"], "valor\n");
        assert_eq!(out.code, 2);
        assert!(store.is_empty());
        assert_eq!(key(&store, &["status", "Gemini"], "").code, 2);
        assert_eq!(key(&store, &["delete", "Gemini"], "").code, 2);
    }

    #[test]
    fn unavailable_keyring_exits_1() {
        for (argv, stdin) in [
            (&["set", "gemini"][..], "valor\n"),
            (&["status", "gemini"][..], ""),
            (&["delete", "gemini"][..], ""),
        ] {
            let out = key(&UnavailableStore, argv, stdin);
            assert_eq!(out.code, 1, "{argv:?}");
            assert!(out.stderr.contains("keyring indisponível"), "{argv:?}");
        }
    }

    #[test]
    fn non_utf8_stdin_exits_2() {
        let store = MemoryStore::default();
        let out = key_bytes(&store, &["set", "gemini"], &[0xff, 0xfe, b'\n']);
        assert_eq!(out.code, 2);
        assert!(store.is_empty());
    }

    #[test]
    fn refused_operation_exits_1() {
        for (argv, stdin) in [
            (&["set", "gemini"][..], "valor\n"),
            (&["status", "gemini"][..], ""),
            (&["delete", "gemini"][..], ""),
        ] {
            let out = key(&RefusingStore, argv, stdin);
            assert_eq!(out.code, 1, "{argv:?}");
            assert!(out.stderr.contains("keyring indisponível"), "{argv:?}");
            assert!(!out.stderr.contains("valor"), "{argv:?}");
        }
    }
}
