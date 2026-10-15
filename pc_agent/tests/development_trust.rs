use std::{net::SocketAddr, num::NonZeroU64, time::Duration};
use tokio::{net::UdpSocket, sync::oneshot, time::timeout};
use virtual_gamepad_agent::{
    config::Config,
    input::LatestState,
    network::Receiver,
    protocol::*,
    session::{AdmissionError, DevelopmentTrust, Handler},
};

fn hello(instance: u64, nonce: u64) -> Message {
    Message::Hello(Hello {
        client_nonce: Nonce(NonZeroU64::new(nonce).unwrap()),
        client_instance: InstanceId(NonZeroU64::new(instance).unwrap()),
        state_rate: 60,
        capabilities: 1,
        app_build: 1,
        api_level: 34,
    })
}
fn source(port: u16) -> SocketAddr {
    ([127, 0, 0, 1], port).into()
}
fn assert_ack(message: Option<Message>, nonce: u64) -> Nonce {
    let Some(Message::HelloAck(ack)) = message else {
        panic!("missing acknowledgement")
    };
    assert_eq!(ack.client_nonce.0.get(), nonce);
    assert_eq!(ack.session, None);
    assert_eq!(
        (
            ack.state_rate,
            ack.timeout_ms,
            ack.flags,
            ack.capacity,
            ack.slot
        ),
        (60, 250, 4, 1, 255)
    );
    assert!(Message::HelloAck(ack).encode().is_ok());
    ack.server_nonce
}

#[test]
fn disabled_is_default_and_release_flag_is_rejected() {
    assert!(!Config::default().manual_trust_first_client);
    let args = || ["--manual-trust-first-client".to_owned()];
    assert_eq!(Config::from_args(args()).is_ok(), cfg!(debug_assertions));
    assert!(Config::from_args([args()[0].clone(), args()[0].clone()]).is_err());
    let mut handler = DevelopmentTrust::new(false);
    assert!(handler.control(hello(1, 1), source(1000)).is_none());
    assert!(handler.status().is_none());
}

#[test]
fn first_instance_claim_second_instance_refusal_and_source_rotation() {
    let mut handler = DevelopmentTrust::new(true);
    let first_nonce = assert_ack(handler.control(hello(1, 1), source(1000)), 1);
    let (status, destination) = handler.status().unwrap();
    assert_eq!(destination, source(1000));
    let Message::Status(status) = status else {
        panic!()
    };
    assert_eq!(status.session, None);
    assert_eq!(status.flags, 0);
    assert_eq!(status.timeout_ms, 250);
    assert!(handler.status().is_none()); // Bounded, no permanent traffic after a hello.
    assert_ack(handler.control(hello(2, 2), source(2000)), 2);
    assert!(handler.status().is_none()); // Second instance cannot claim the destination.
    let rotated_nonce = assert_ack(handler.control(hello(1, 3), source(3000)), 3);
    assert_ne!(first_nonce, rotated_nonce);
    assert_eq!(handler.status().unwrap().1, source(3000));
    assert_ack(handler.control(hello(1, 4), source(3000)), 4);
    assert_eq!(handler.status().unwrap().1, source(3000));
}

#[test]
fn no_current_or_old_session_can_apply_state_and_pairing_is_inactive() {
    let mut handler = DevelopmentTrust::new(true);
    handler.control(hello(1, 1), source(1000));
    handler.control(hello(1, 2), source(2000));
    let bytes = include_bytes!("../../protocol/fixtures/v1/mixed.bin");
    let Message::ControllerState(state) = decode(bytes).unwrap() else {
        panic!()
    };
    for source in [source(1000), source(2000), source(3000)] {
        assert_eq!(handler.admit(&state, source), Err(AdmissionError::Refused));
    }
    let request = decode(include_bytes!(
        "../../protocol/fixtures/v1/pair_request.bin"
    ))
    .unwrap();
    assert!(handler.control(request, source(2000)).is_none());
}

#[tokio::test]
async fn real_udp_ack_and_presession_status_use_listener_socket_and_shutdown_clears() {
    let server = UdpSocket::bind(source(0)).await.unwrap();
    let address = server.local_addr().unwrap();
    let client = UdpSocket::bind(source(0)).await.unwrap();
    let (latest, state) = LatestState::channel();
    let (receiver, _) = Receiver::new(server, DevelopmentTrust::new(true), latest);
    let (stop, stopped) = oneshot::channel();
    let task = tokio::spawn(receiver.run(async {
        let _ = stopped.await;
    }));
    client
        .send_to(&hello(1, 12).encode().unwrap(), address)
        .await
        .unwrap();
    let mut bytes = [0; 64];
    let (length, from) = timeout(Duration::from_secs(2), client.recv_from(&mut bytes))
        .await
        .unwrap()
        .unwrap();
    assert_eq!(from, address);
    assert_ack(Some(decode(&bytes[..length]).unwrap()), 12);
    let (length, from) = timeout(Duration::from_secs(2), client.recv_from(&mut bytes))
        .await
        .unwrap()
        .unwrap();
    assert_eq!(from, address);
    assert!(matches!(
        decode(&bytes[..length]),
        Ok(Message::Status(Status {
            session: None,
            flags: 0,
            ..
        }))
    ));
    stop.send(()).unwrap();
    let counters = timeout(Duration::from_secs(2), task)
        .await
        .unwrap()
        .unwrap()
        .unwrap();
    assert_eq!(counters.accepted, 0);
    assert_eq!(counters.replies, 2);
    assert!(state.borrow().is_none());
    assert!(UdpSocket::bind(address).await.is_ok());
}
