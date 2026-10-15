//! Bounded trusted-LAN pairing policy. No sockets, disk I/O or backend calls.
use crate::protocol::{Hello, InstanceId, Nonce, PairRequest, RejectReason};
use std::{
    net::{IpAddr, SocketAddr},
    time::Duration,
};
use tokio::time::Instant;

pub const WINDOW: Duration = Duration::from_secs(120);
const CHALLENGE: Duration = Duration::from_secs(10);
const COOLDOWN: Duration = Duration::from_secs(60);
const MAX_CHALLENGES: usize = 32;
const MAX_FAILURES: usize = 64;

#[derive(Clone, Copy)]
struct Challenge {
    hello: Hello,
    server: Nonce,
    source: SocketAddr,
    expires: Instant,
}
#[derive(Clone, Copy, PartialEq, Eq)]
enum FailureKey {
    Source(IpAddr),
    Client(InstanceId),
}
struct Failure {
    key: FailureKey,
    count: u8,
    expires: Instant,
}

/// Intentionally does not implement Debug: the window contains the visible secret.
pub struct PairingPolicy {
    paired: Option<InstanceId>,
    window: Option<(u32, Instant)>,
    challenges: Vec<Challenge>,
    failures: Vec<Failure>,
}
impl PairingPolicy {
    pub fn new(paired: Option<InstanceId>) -> Self {
        Self {
            paired,
            window: None,
            challenges: Vec::new(),
            failures: Vec::new(),
        }
    }
    pub fn paired(&self) -> Option<InstanceId> {
        self.paired
    }
    pub fn window(&self, now: Instant) -> Option<(u32, Instant)> {
        self.window.filter(|(_, expires)| now < *expires)
    }
    pub fn close_window(&mut self) {
        self.window = None;
        self.challenges.clear();
    }
    pub fn open(&mut self, code: u32, now: Instant) -> bool {
        if self.paired.is_some() || code > 999_999 {
            return false;
        }
        self.window = Some((code, now + WINDOW));
        self.challenges.clear();
        true
    }
    pub fn revoke(&mut self) {
        self.paired = None;
        self.window = None;
        self.challenges.clear();
    }
    pub fn commit(&mut self, client: InstanceId) {
        self.paired = Some(client);
        self.window = None;
        self.challenges.clear();
    }
    fn prune(&mut self, now: Instant) {
        self.challenges.retain(|c| now < c.expires);
        self.failures.retain(|f| now < f.expires);
        if self.window.is_some_and(|(_, expires)| now >= expires) {
            self.window = None;
            self.challenges.clear();
        }
    }
    pub fn challenge(
        &mut self,
        hello: Hello,
        source: SocketAddr,
        nonce: Nonce,
        now: Instant,
    ) -> Option<Nonce> {
        self.prune(now);
        if self.paired.is_some() || self.window.is_none() {
            return None;
        }
        if let Some(c) = self
            .challenges
            .iter()
            .find(|c| c.source == source && c.hello == hello)
        {
            return Some(c.server);
        }
        self.challenges
            .retain(|c| c.hello.client_instance != hello.client_instance);
        if self.challenges.len() >= MAX_CHALLENGES {
            return None;
        }
        self.challenges.push(Challenge {
            hello,
            source,
            server: nonce,
            expires: now + CHALLENGE,
        });
        Some(nonce)
    }
    /// A successful request reserves this window once. Persistence must precede commit.
    pub fn reserve(
        &mut self,
        request: PairRequest,
        source: SocketAddr,
        now: Instant,
    ) -> Result<Hello, (RejectReason, u8)> {
        self.prune(now);
        let keys = [
            FailureKey::Source(source.ip()),
            FailureKey::Client(request.client_instance),
        ];
        if let Some(f) = self
            .failures
            .iter()
            .filter(|f| keys.contains(&f.key) && f.count >= 5)
            .max_by_key(|f| f.expires)
        {
            return Err((
                RejectReason::RateLimited,
                f.expires
                    .duration_since(now)
                    .as_secs()
                    .saturating_add(1)
                    .min(60) as u8,
            ));
        }
        if self.paired.is_some() {
            return Err((RejectReason::NoSlot, 0));
        }
        let Some((code, _)) = self.window else {
            return Err((RejectReason::WindowClosed, 0));
        };
        let challenge = self
            .challenges
            .iter()
            .find(|c| {
                c.source == source
                    && c.hello.client_instance == request.client_instance
                    && c.hello.client_nonce == request.client_nonce
                    && c.server == request.server_nonce
            })
            .copied();
        if code != request.code || challenge.is_none() || request.slot != 255 {
            let missing = keys
                .iter()
                .filter(|key| !self.failures.iter().any(|f| f.key == **key))
                .count();
            if self.failures.len() + missing > MAX_FAILURES {
                return Err((RejectReason::RateLimited, 60));
            }
            let mut limited = false;
            for key in keys {
                let index = if let Some(i) = self.failures.iter().position(|f| f.key == key) {
                    i
                } else {
                    self.failures.push(Failure {
                        key,
                        count: 0,
                        expires: now + COOLDOWN,
                    });
                    self.failures.len() - 1
                };
                let f = &mut self.failures[index];
                f.count = f.count.saturating_add(1);
                if f.count >= 5 {
                    f.expires = now + COOLDOWN;
                    limited = true;
                }
            }
            return Err((
                if limited {
                    RejectReason::RateLimited
                } else {
                    RejectReason::IncorrectCode
                },
                if limited { 60 } else { 0 },
            ));
        }
        // Saturated failure storage also refuses new valid guesses until capacity frees.
        if self.failures.len() >= MAX_FAILURES
            && !self.failures.iter().any(|f| keys.contains(&f.key))
        {
            return Err((RejectReason::RateLimited, 60));
        }
        self.window = None;
        self.challenges.clear();
        Ok(challenge.expect("checked challenge").hello)
    }
}

pub fn random_code() -> Result<u32, getrandom::Error> {
    loop {
        let mut bytes = [0; 4];
        getrandom::fill(&mut bytes)?;
        let value = u32::from_ne_bytes(bytes);
        // Rejection sampling avoids modulo bias across one million possible codes.
        if value < u32::MAX - u32::MAX % 1_000_000 {
            return Ok(value % 1_000_000);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::num::NonZeroU64;
    fn hello(id: u64) -> Hello {
        Hello {
            client_nonce: Nonce(NonZeroU64::new(1).unwrap()),
            client_instance: InstanceId(NonZeroU64::new(id).unwrap()),
            state_rate: 60,
            capabilities: 3,
            app_build: 1,
            api_level: 34,
        }
    }
    fn request(id: u64, code: u32) -> PairRequest {
        PairRequest {
            client_nonce: hello(id).client_nonce,
            client_instance: hello(id).client_instance,
            server_nonce: Nonce(NonZeroU64::new(2).unwrap()),
            code,
            slot: 255,
        }
    }
    fn source(port: u16) -> SocketAddr {
        ([127, 0, 0, 1], port).into()
    }
    #[tokio::test(start_paused = true)]
    async fn reservation_consumes_window_and_requires_explicit_replacement() {
        let mut p = PairingPolicy::new(None);
        let now = Instant::now();
        assert!(p.open(123456, now));
        let r = request(1, 123456);
        p.challenge(hello(1), source(1), r.server_nonce, now)
            .unwrap();
        assert_eq!(
            p.reserve(r, source(2), now),
            Err((RejectReason::IncorrectCode, 0))
        );
        assert_eq!(p.reserve(r, source(1), now), Ok(hello(1)));
        assert_eq!(
            p.reserve(r, source(1), now),
            Err((RejectReason::WindowClosed, 0))
        );
        p.commit(hello(1).client_instance);
        assert!(!p.open(123456, now));
        assert_eq!(
            p.reserve(request(2, 123456), source(2), now),
            Err((RejectReason::NoSlot, 0))
        );
        p.revoke();
        assert!(p.open(654321, now));
        assert_eq!(
            p.reserve(r, source(1), now),
            Err((RejectReason::IncorrectCode, 0))
        );
    }
    #[tokio::test(start_paused = true)]
    async fn cooldown_survives_port_and_identity_changes_and_window_reopen() {
        let now = Instant::now();
        let mut p = PairingPolicy::new(None);
        p.open(123456, now);
        for i in 1..=5 {
            let result = p.reserve(request(i, 0), source(i as u16), now);
            assert_eq!(
                result.unwrap_err().0,
                if i == 5 {
                    RejectReason::RateLimited
                } else {
                    RejectReason::IncorrectCode
                }
            );
        }
        p.open(123456, now);
        assert_eq!(
            p.reserve(request(20, 123456), source(50), now),
            Err((RejectReason::RateLimited, 60))
        );
        let later = now + COOLDOWN;
        let r = request(20, 123456);
        p.challenge(hello(20), source(50), r.server_nonce, later);
        assert!(p.reserve(r, source(50), later).is_ok());
    }
    #[tokio::test(start_paused = true)]
    async fn challenge_and_window_expiry_are_independent_and_bounded() {
        let now = Instant::now();
        let mut p = PairingPolicy::new(None);
        p.open(123456, now);
        let r = request(1, 123456);
        for i in 1..=32 {
            assert!(
                p.challenge(hello(i), source(i as u16), r.server_nonce, now)
                    .is_some()
            );
        }
        assert!(
            p.challenge(hello(33), source(33), r.server_nonce, now)
                .is_none()
        );
        assert_eq!(
            p.reserve(r, source(1), now + CHALLENGE),
            Err((RejectReason::IncorrectCode, 0))
        );
        p.challenge(
            hello(1),
            source(1),
            r.server_nonce,
            now + WINDOW - Duration::from_secs(1),
        );
        assert_eq!(
            p.reserve(r, source(1), now + WINDOW),
            Err((RejectReason::WindowClosed, 0))
        );
    }
}
