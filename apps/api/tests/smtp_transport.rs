use horizon_api::auth::{EmailSender, SmtpEmailSender};
use std::{
    io::{BufRead, BufReader, Write},
    net::TcpListener,
    thread,
    time::Duration,
};

#[actix_web::test]
async fn smtp_submission_on_port_587_uses_starttls() {
    let listener = TcpListener::bind("127.0.0.1:0").expect("test SMTP listener should bind");
    let port = listener
        .local_addr()
        .expect("test SMTP listener should have an address")
        .port();
    let server = thread::spawn(move || {
        let (mut stream, _) = listener
            .accept()
            .expect("SMTP sender should connect to the test server");
        stream
            .set_read_timeout(Some(Duration::from_secs(2)))
            .expect("test SMTP read timeout should be set");
        stream
            .write_all(b"220 test SMTP ready\r\n")
            .expect("test SMTP greeting should be sent");

        let mut reader = BufReader::new(stream.try_clone().expect("stream should clone"));
        let mut ehlo = String::new();
        reader
            .read_line(&mut ehlo)
            .expect("SMTP sender should send EHLO before TLS");
        assert!(
            ehlo.starts_with("EHLO "),
            "expected EHLO, received {ehlo:?}"
        );

        stream
            .write_all(b"250-test SMTP\r\n250-STARTTLS\r\n250 AUTH PLAIN LOGIN\r\n")
            .expect("test SMTP capabilities should be sent");
        let mut starttls = String::new();
        reader
            .read_line(&mut starttls)
            .expect("SMTP sender should request STARTTLS");
        assert_eq!(starttls.trim(), "STARTTLS");
        stream
            .write_all(b"220 begin TLS\r\n")
            .expect("test SMTP server should accept STARTTLS");
    });

    std::env::set_var("SMTP_HOST", "127.0.0.1");
    std::env::set_var("SMTP_PORT", port.to_string());
    std::env::set_var("SMTP_TLS", "true");
    std::env::set_var("SMTP_FROM", "Horizon <sender@example.test>");
    std::env::remove_var("SMTP_USERNAME");
    std::env::remove_var("SMTP_PASSWORD");

    let sender = SmtpEmailSender::from_env().expect("SMTP settings should be valid");
    let result = sender
        .send_magic_link("recipient@example.test", "https://horizon.test/#token=test")
        .await;
    assert!(
        result.is_err(),
        "the test server intentionally stops after STARTTLS"
    );
    server
        .join()
        .expect("SMTP server should observe the STARTTLS flow");
}
