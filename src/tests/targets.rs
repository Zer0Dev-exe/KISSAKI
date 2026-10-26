use super::*;
use std::io;

#[test]
fn combines_arguments_lists_and_stdin_in_order() -> Result<()> {
    let mut targets = Targets::new(3, false);
    targets.argument("https://example.com/first#fragment")?;
    targets.read(b"# ignored\n\n  https://example.com/second  \r\n".as_slice())?;
    targets.read(b"https://example.com/third\n".as_slice())?;
    let urls = targets.finish()?;
    assert_eq!(
        urls.iter().map(Url::path).collect::<Vec<_>>(),
        ["/first", "/second", "/third"]
    );
    Ok(())
}

#[test]
fn enforces_aggregate_bytes_across_sources_including_comments() -> Result<()> {
    let mut targets = Targets::new(1, false);
    let seed = "https://example.com/";
    targets.argument(seed)?;
    let padding = format!("#{}", " ".repeat(MAX_INPUT_BYTES - seed.len() - 1));
    targets.read(padding.as_bytes())?;
    assert_eq!(targets.bytes, MAX_INPUT_BYTES);
    assert!(
        targets
            .read(b"\n".as_slice())
            .is_err_and(|error| error.to_string().contains("combined target input"))
    );
    assert!(targets.argument(seed).is_err());
    assert_eq!(targets.finish()?.len(), 1);
    Ok(())
}

#[test]
fn stops_reading_after_the_shared_byte_budget() {
    let mut targets = Targets::new(1, false);
    let mut reader = io::repeat(b' ').take((MAX_INPUT_BYTES * 2) as u64);
    assert!(targets.read(&mut reader).is_err());
    assert_eq!(reader.limit(), (MAX_INPUT_BYTES - 1) as u64);
    assert_eq!(targets.bytes, 0);
}

#[test]
fn enforces_seed_count_while_reading_and_before_retention() -> Result<()> {
    let seed = "https://example.com/";
    let mut targets = Targets::new(1, false);
    targets.argument(seed)?;
    assert!(
        targets
            .read(format!("{seed}\n").as_bytes())
            .is_err_and(|error| error.to_string().contains("seed count exceeds max-pages"))
    );
    assert_eq!(targets.finish()?.len(), 1);
    Ok(())
}

#[test]
fn rejects_empty_malformed_and_disallowed_inputs() -> Result<()> {
    let mut empty = Targets::new(1, false);
    empty.read(b"# no targets\n \n".as_slice())?;
    assert!(empty.finish().is_err());
    for value in [
        "file:///etc/passwd",
        "https://user:secret@example.com/",
        "http://127.0.0.1/",
        "not a URL",
    ] {
        assert!(Targets::new(1, false).argument(value).is_err(), "{value}");
    }
    assert!(Targets::new(1, false).read([0xff].as_slice()).is_err());
    let mut private = Targets::new(1, true);
    private.argument("http://127.0.0.1/")?;
    assert_eq!(private.finish()?.len(), 1);
    Ok(())
}
