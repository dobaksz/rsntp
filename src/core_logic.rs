use crate::error::{KissCode, ProtocolError, SynchronizationError};
use crate::packet::{LeapIndicator, Mode, Packet, ReferenceIdentifier, SntpTimestamp};
use crate::result::SynchronizationResult;
use std::time::SystemTime;

pub struct Request {
    packet: Packet,
}

impl Request {
    pub fn new() -> Request {
        Self::new_with_transmit_time(SystemTime::now())
    }

    pub fn new_with_transmit_time(transmit_time: SystemTime) -> Request {
        Request {
            packet: Packet {
                li: LeapIndicator::NoWarning,
                mode: Mode::Client,
                stratum: 0,
                reference_identifier: ReferenceIdentifier::Empty,
                reference_timestamp: SntpTimestamp::zero(),
                originate_timestamp: SntpTimestamp::zero(),
                receive_timestamp: SntpTimestamp::zero(),
                transmit_timestamp: SntpTimestamp::from_systemtime(transmit_time),
            },
        }
    }

    pub fn as_bytes(&self) -> [u8; Packet::ENCODED_LEN] {
        self.packet.to_bytes()
    }

    fn into_packet(self) -> Packet {
        self.packet
    }
}

pub struct Reply {
    request: Packet,
    reply: Packet,
    reply_timestamp: SntpTimestamp,
}

impl Reply {
    pub fn new(request: Request, reply: Packet) -> Reply {
        Self::new_with_reply_time(request, reply, SystemTime::now())
    }

    pub fn new_with_reply_time(request: Request, reply: Packet, reply_time: SystemTime) -> Reply {
        Reply {
            request: request.into_packet(),
            reply,
            reply_timestamp: SntpTimestamp::from_systemtime(reply_time),
        }
    }

    fn check(&self) -> Result<(), ProtocolError> {
        if self.reply.stratum == 0 {
            return Err(ProtocolError::KissODeath(KissCode::new(
                &self.reply.reference_identifier,
            )));
        }

        if self.reply.originate_timestamp != self.request.transmit_timestamp {
            return Err(ProtocolError::InvalidOriginateTimestamp);
        }

        if self.reply.transmit_timestamp.is_zero() {
            return Err(ProtocolError::InvalidTransmitTimestamp);
        }

        if self.reply.mode != Mode::Server && self.reply.mode != Mode::Broadcast {
            return Err(ProtocolError::InvalidMode);
        }
        Ok(())
    }

    pub fn process(self) -> Result<SynchronizationResult, SynchronizationError> {
        self.check()?;

        let originate_ts = self.reply.originate_timestamp;
        let transmit_ts = self.reply.transmit_timestamp;
        let receive_ts = self.reply.receive_timestamp;
        let round_trip_delay_s = (self.reply_timestamp - originate_ts) - (transmit_ts - receive_ts);
        let clock_offset_s =
            ((receive_ts - originate_ts) + (transmit_ts - self.reply_timestamp)) / 2.0;
        Ok(SynchronizationResult::new(
            clock_offset_s,
            round_trip_delay_s,
            self.reply.reference_identifier.clone(),
            self.reply.li,
            self.reply.stratum,
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    macro_rules! assert_between {
        ($var: expr, $lower: expr, $upper: expr) => {
            if $var < $lower || $var > $upper {
                panic!(
                    "Assertion failed, {:?} is not between {:?} and {:?}",
                    $var, $lower, $upper
                );
            }
        };
    }

    #[test]
    fn basic_synchronization_works() {
        let now = SystemTime::now();
        let request = Request::new_with_transmit_time(now);

        let reply_packet = Packet {
            li: LeapIndicator::NoWarning,
            mode: Mode::Server,
            stratum: 1,
            reference_identifier: ReferenceIdentifier::new_ascii([0x4c, 0x4f, 0x43, 0x4c]).unwrap(),
            reference_timestamp: SntpTimestamp::from_systemtime(
                now - std::time::Duration::from_secs(86400),
            ),
            originate_timestamp: request.packet.transmit_timestamp,
            receive_timestamp: SntpTimestamp::from_systemtime(
                now - std::time::Duration::from_millis(400),
            ),
            transmit_timestamp: SntpTimestamp::from_systemtime(
                now - std::time::Duration::from_millis(400),
            ),
        };

        let reply = Reply::new_with_reply_time(
            request,
            reply_packet,
            now + std::time::Duration::from_millis(200),
        );

        let result = reply.process().unwrap();

        assert_between!(result.clock_offset().as_secs_f64(), -0.51, -0.49);
        assert_between!(result.round_trip_delay().as_secs_f64(), 0.19, 0.21);

        assert_eq!(result.reference_identifier().to_string(), "LOCL");
        assert_eq!(result.leap_indicator(), LeapIndicator::NoWarning);
        assert_eq!(result.stratum(), 1);
    }

    #[test]
    fn sync_fails_if_reply_originate_ts_does_not_match_request_transmit_ts() {
        let request = Request::new();
        let now = SystemTime::now();

        let reply_packet = Packet {
            li: LeapIndicator::NoWarning,
            mode: Mode::Server,
            stratum: 1,
            reference_identifier: ReferenceIdentifier::new_ascii([0x4c, 0x4f, 0x43, 0x4c]).unwrap(),
            reference_timestamp: SntpTimestamp::from_systemtime(
                now - std::time::Duration::from_secs(86400),
            ),
            originate_timestamp: SntpTimestamp::from_systemtime(now),
            receive_timestamp: SntpTimestamp::from_systemtime(
                now - std::time::Duration::from_millis(500),
            ),
            transmit_timestamp: SntpTimestamp::from_systemtime(
                now - std::time::Duration::from_millis(500),
            ),
        };

        let reply = Reply::new(request, reply_packet);

        let result = reply.process();

        assert!(result.is_err());
    }

    #[test]
    fn sync_fails_if_reply_contains_zero_transmit_timestamp() {
        let request = Request::new();
        let now = SystemTime::now();

        let reply_packet = Packet {
            li: LeapIndicator::NoWarning,
            mode: Mode::Server,
            stratum: 1,
            reference_identifier: ReferenceIdentifier::new_ascii([0x4c, 0x4f, 0x43, 0x4c]).unwrap(),
            reference_timestamp: SntpTimestamp::from_systemtime(
                now - std::time::Duration::from_secs(86400),
            ),
            originate_timestamp: request.packet.transmit_timestamp,
            receive_timestamp: SntpTimestamp::from_systemtime(
                now - std::time::Duration::from_millis(500),
            ),
            transmit_timestamp: SntpTimestamp::zero(),
        };

        let reply = Reply::new(request, reply_packet);

        let result = reply.process();

        assert!(result.is_err());
    }

    #[test]
    fn sync_fails_if_reply_contains_wrong_mode() {
        let request = Request::new();
        let now = SystemTime::now();

        let reply_packet = Packet {
            li: LeapIndicator::NoWarning,
            mode: Mode::Client,
            stratum: 1,
            reference_identifier: ReferenceIdentifier::new_ascii([0x4c, 0x4f, 0x43, 0x4c]).unwrap(),
            reference_timestamp: SntpTimestamp::from_systemtime(
                now - std::time::Duration::from_secs(86400),
            ),
            originate_timestamp: request.packet.transmit_timestamp,
            receive_timestamp: SntpTimestamp::from_systemtime(
                now - std::time::Duration::from_millis(500),
            ),
            transmit_timestamp: SntpTimestamp::from_systemtime(
                now - std::time::Duration::from_millis(500),
            ),
        };

        let reply = Reply::new(request, reply_packet);

        let result = reply.process();

        assert!(result.is_err());
    }

    #[test]
    fn sync_fails_if_kiss_o_death_received() {
        let request = Request::new();
        let now = SystemTime::now();

        let reply_packet = Packet {
            li: LeapIndicator::NoWarning,
            mode: Mode::Server,
            stratum: 0,
            reference_identifier: ReferenceIdentifier::new_ascii([0x52, 0x41, 0x54, 0x45]).unwrap(),
            reference_timestamp: SntpTimestamp::from_systemtime(
                now - std::time::Duration::from_secs(86400),
            ),
            originate_timestamp: request.packet.transmit_timestamp,
            receive_timestamp: SntpTimestamp::from_systemtime(
                now - std::time::Duration::from_millis(500),
            ),
            transmit_timestamp: SntpTimestamp::from_systemtime(
                now - std::time::Duration::from_millis(500),
            ),
        };

        let reply = Reply::new(request, reply_packet);

        let err = reply.process().unwrap_err();

        if let SynchronizationError::ProtocolError(ProtocolError::KissODeath(
            KissCode::RateExceeded,
        )) = err
        {
            // pass
        } else {
            panic!("Wrong error received");
        }
    }
}
