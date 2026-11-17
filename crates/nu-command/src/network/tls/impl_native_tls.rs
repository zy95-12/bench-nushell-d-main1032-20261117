use nu_protocol::ShellError;

#[doc = include_str!("./tls.rustdoc.md")]
pub fn tls(allow_insecure: bool) -> Result<ureq::tls::TlsConfig, ShellError> {
    Ok(ureq::tls::TlsConfig::builder()
        .provider(ureq::tls::TlsProvider::NativeTls)
        .root_certs(ureq::tls::RootCerts::PlatformVerifier)
        .disable_verification(allow_insecure)
        .build())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tls_uses_platform_roots_and_explicit_insecure_option() {
        let secure = tls(false).unwrap();
        assert_eq!(secure.provider(), ureq::tls::TlsProvider::NativeTls);
        assert!(matches!(
            secure.root_certs(),
            ureq::tls::RootCerts::PlatformVerifier
        ));
        assert!(!secure.disable_verification());
        assert!(tls(true).unwrap().disable_verification());
    }
}
