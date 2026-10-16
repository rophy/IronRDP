use ironrdp_acceptor::credssp::client_error_code;
use ironrdp_connector::sspi;
use ironrdp_connector::sspi::credssp::NStatusCode;

#[test]
fn failed_logon_is_reported_as_logon_failure() {
    for kind in [sspi::ErrorKind::MessageAltered, sspi::ErrorKind::LogonDenied] {
        let error = sspi::Error::new(kind, "test");
        assert_eq!(client_error_code(&error), Some(NStatusCode::LOGON_FAILURE));
    }
}

#[test]
fn other_errors_keep_the_sspi_code() {
    let error = sspi::Error::new(sspi::ErrorKind::InternalError, "test");
    assert_eq!(client_error_code(&error), None);
}
