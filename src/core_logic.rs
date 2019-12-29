use crate::error::{KissCode, ProtocolError, SynchroniztationError};
use crate::packet::{LeapIndicator, Mode, Packet, ReferenceIdentifier, SntpTimestamp};
use crate::transport::Transport;
use chrono::{DateTime, Duration, Utc};

/// Results of a synchronization.
///
/// If you just simply need a fairly accurate SNTP time then check the `datetime()` method. Other methods
/// provide more detailed information received from the server and might need deeper knwoledge about
/// SNTP protocol internals.
#[derive(Debug, Clone)]
pub struct SynchronizationResult {
  clock_offset: Duration,
  round_trip_delay: Duration,
  reference_identifier: ReferenceIdentifier,
  leap_indicator: LeapIndicator,
  stratum: u8,
}

impl SynchronizationResult {
  /// Returns with the offset between the server and local clock.
  ///
  /// It is a signed duration, negative value means the local clock is ahead.
  ///
  /// # Example
  ///
  /// Print the synchronized local time using clock offset:
  /// ```no_run
  /// use rsntp::SntpClient;
  /// use chrono::Local;
  ///
  /// let client = SntpClient::new("pool.ntp.org").unwrap();
  /// let result = client.synchronize().unwrap();
  ///
  /// println!("Local time: {}", Local::now() + result.clock_offset());
  /// ```
  pub fn clock_offset(&self) -> Duration {
    self.clock_offset
  }

  /// Return with the round trip delay
  ///
  /// The time is needed for SNTP packets to travel back and forth between the host and the server.
  /// It is a signed value but negative values should not be possible in client mode
  /// (which is currently always used by the library).
  ///
  /// # Example
  ///
  /// ```no_run
  /// use rsntp::SntpClient;
  /// use chrono::Local;
  ///
  /// let client = SntpClient::new("pool.ntp.org").unwrap();
  /// let result = client.synchronize().unwrap();
  ///
  /// println!("RTT: {} ms", result.round_trip_delay().num_milliseconds());
  /// ```
  pub fn round_trip_delay(&self) -> Duration {
    self.round_trip_delay
  }

  /// Returns with the server reference identifier.
  ///
  /// This identifies the particular reference source. For primary server (startum = 1) this is a four
  /// byte ASCII string, for secondary IPv4 servers (startum >= 2) this is an IP address.
  ///   
  /// # Example
  ///
  /// ```no_run
  /// use rsntp::SntpClient;
  /// use chrono::Local;
  ///
  /// let client = SntpClient::new("pool.ntp.org").unwrap();
  /// let result = client.synchronize().unwrap();
  ///
  /// println!("Server reference identifier: {}", result.reference_identifier());
  /// ```
  pub fn reference_identifier(&self) -> &ReferenceIdentifier {
    &self.reference_identifier
  }

  /// Returns with the current UTC date and time, based on the synchronized SNTP timestamp.
  ///
  /// This is the current UTC date and time, calulcated by adding clock offset the UTC time. To be accurate,
  /// use the returned value immediately after the call of this function.
  ///
  /// # Example
  ///
  /// Calcuating synchronized local time:
  /// ```no_run
  /// use rsntp::SntpClient;
  /// use chrono::{DateTime, Local};
  ///
  /// let client = SntpClient::new("pool.ntp.org").unwrap();
  /// let result = client.synchronize().unwrap();
  ///
  /// let local_time: DateTime<Local> = DateTime::from(result.datetime());
  /// ```
  pub fn datetime(&self) -> DateTime<Utc> {
    Utc::now() + self.clock_offset
  }

  /// Returns with the leap indicator
  ///
  /// This is the leap indicator returned by the server. It is a warning of an impending leap second to be
  /// inserted/deleted in the last minute of the current day.
  ///
  /// It is set before 23:59 on the day of insertion and reset after 00:00 on the following day. This causes
  /// the number of seconds (rollover interval) in the day of insertion to be increased or decreased by one.
  ///
  /// # Example
  ///
  /// Printing leap indicator:
  ///
  /// ```no_run
  /// use rsntp::SntpClient;
  /// use chrono::{DateTime, Local};
  ///
  /// let client = SntpClient::new("pool.ntp.org").unwrap();
  /// let result = client.synchronize().unwrap();
  ///
  /// println!("Leap indicator: {:?}", result.leap_indicator());
  /// ```
  pub fn leap_indicator(&self) -> LeapIndicator {
    self.leap_indicator
  }

  /// Returns with the server stratum
  ///
  /// This is the number indicating the startum of the server clock with values defined as:
  /// *  1 - Primary reference (e.g., calibrated atomic clock, radio clock, etc...)
  /// *  2..15 - Secondary reference (via NTP, calculated as the stratum of system peer plus one)
  /// *  16 - Unsynchronized
  /// *  16..255 - Reserved
  ///
  /// # Example
  ///
  /// ```no_run
  /// use rsntp::SntpClient;
  /// use chrono::{DateTime, Local};
  ///
  /// let client = SntpClient::new("pool.ntp.org").unwrap();
  /// let result = client.synchronize().unwrap();
  ///
  /// assert!(result.stratum() >= 1);
  /// ```
  pub fn stratum(&self) -> u8 {
    self.stratum
  }
}

fn check_reply_validity(request: &Packet, reply: &Packet) -> Result<(), ProtocolError> {
  if reply.stratum == 0 {
    return Err(ProtocolError::KissODeath(KissCode::new(
      &reply.reference_identifier,
    )));
  }

  if reply.originate_timestamp != request.transmit_timestamp {
    return Err(ProtocolError::InvalidOriginateTimestamp);
  }

  if reply.transmit_timestamp.is_zero() {
    return Err(ProtocolError::InvalidTransmitTimestamp);
  }

  if reply.mode != Mode::Server && reply.mode != Mode::Broadcast {
    return Err(ProtocolError::InvalidMode);
  }

  Ok(())
}

fn calculate_timeinfo(
  reply: &Packet,
  destination_timestamp: DateTime<Utc>,
) -> Result<SynchronizationResult, SynchroniztationError> {
  let originate_ts = reply.originate_timestamp.to_datetime();
  let transmit_ts = reply.transmit_timestamp.to_datetime();
  let receive_ts = reply.receive_timestamp.to_datetime();

  let round_trip_delay = (destination_timestamp - originate_ts) - (transmit_ts - receive_ts);
  let clock_offset = ((receive_ts - originate_ts) + (transmit_ts - destination_timestamp)) / 2;

  Ok(SynchronizationResult {
    round_trip_delay,
    clock_offset,
    reference_identifier: reply.reference_identifier.clone(),
    leap_indicator: reply.li,
    stratum: reply.stratum,
  })
}

pub fn synchronize<T>(transport: &mut T) -> Result<SynchronizationResult, SynchroniztationError>
where
  T: Transport,
{
  let request = Packet {
    li: LeapIndicator::NoWarning,
    mode: Mode::Client,
    stratum: 0,
    reference_identifier: ReferenceIdentifier::Empty,
    reference_timestamp: SntpTimestamp::zero(),
    originate_timestamp: SntpTimestamp::zero(),
    receive_timestamp: SntpTimestamp::zero(),
    transmit_timestamp: SntpTimestamp::from_datetime(Utc::now()),
  };

  transport.send(&request)?;
  let reply = transport.receive()?;

  let destination_timestamp = Utc::now();

  check_reply_validity(&request, &reply)?;
  calculate_timeinfo(&reply, destination_timestamp)
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

  struct ServerMock<F>
  where
    F: Fn(&Packet) -> Result<Result<Packet, SynchroniztationError>, SynchroniztationError>,
  {
    handler: F,
    answer: Option<Result<Packet, SynchroniztationError>>,
  }

  impl<F> ServerMock<F>
  where
    F: Fn(&Packet) -> Result<Result<Packet, SynchroniztationError>, SynchroniztationError>,
  {
    fn new(handler: F) -> ServerMock<F> {
      ServerMock {
        handler,
        answer: None,
      }
    }
  }

  impl<F> Transport for ServerMock<F>
  where
    F: Fn(&Packet) -> Result<Result<Packet, SynchroniztationError>, SynchroniztationError>,
  {
    fn send(&mut self, packet: &Packet) -> Result<(), SynchroniztationError> {
      self.answer = Some((self.handler)(packet)?);

      Ok(())
    }

    fn receive(&mut self) -> Result<Packet, SynchroniztationError> {
      self.answer.take().unwrap()
    }
  }

  #[test]
  fn basic_synchronization_works() {
    let mut server_mock = ServerMock::new(|packet: &Packet| {
      std::thread::sleep(Duration::milliseconds(100).to_std().unwrap());
      let now = Utc::now();
      std::thread::sleep(Duration::milliseconds(100).to_std().unwrap());

      Ok(Ok(Packet {
        li: LeapIndicator::NoWarning,
        mode: Mode::Server,
        stratum: 1,
        reference_identifier: ReferenceIdentifier::new_ascii([0x4c, 0x4f, 0x43, 0x4c]).unwrap(),
        reference_timestamp: SntpTimestamp::from_datetime(now - Duration::days(1)),
        originate_timestamp: packet.transmit_timestamp,
        receive_timestamp: SntpTimestamp::from_datetime(now - Duration::milliseconds(500)),
        transmit_timestamp: SntpTimestamp::from_datetime(now - Duration::milliseconds(500)),
      }))
    });

    let result = synchronize(&mut server_mock).unwrap();

    assert_between!(result.clock_offset().num_milliseconds(), -510, -490);
    assert_between!(result.round_trip_delay().num_milliseconds(), 190, 210);

    assert_eq!(result.reference_identifier().to_string(), "LOCL");
    assert_eq!(result.leap_indicator(), LeapIndicator::NoWarning);
    assert_eq!(result.stratum(), 1);
  }

  #[test]
  fn synch_fails_in_case_of_send_error() {
    let mut server_mock = ServerMock::new(|_: &Packet| {
      Err(SynchroniztationError::IOError(std::io::Error::new(
        std::io::ErrorKind::Other,
        "Test error",
      )))
    });

    let sync_err = synchronize(&mut server_mock);

    assert!(sync_err.is_err());
  }
  #[test]
  fn sync_fails_in_case_of_receive_error() {
    let mut server_mock = ServerMock::new(|_: &Packet| {
      Ok(Err(SynchroniztationError::IOError(std::io::Error::new(
        std::io::ErrorKind::Other,
        "Test error",
      ))))
    });

    let sync_err = synchronize(&mut server_mock);

    assert!(sync_err.is_err());
  }

  #[test]
  fn sync_fails_if_reply_originate_ts_does_not_match_request_transmit_ts() {
    let mut server_mock = ServerMock::new(|_| {
      let now = Utc::now();

      Ok(Ok(Packet {
        li: LeapIndicator::NoWarning,
        mode: Mode::Server,
        stratum: 1,
        reference_identifier: ReferenceIdentifier::new_ascii([0x4c, 0x4f, 0x43, 0x4c]).unwrap(),
        reference_timestamp: SntpTimestamp::from_datetime(now - Duration::days(1)),
        originate_timestamp: SntpTimestamp::from_datetime(now),
        receive_timestamp: SntpTimestamp::from_datetime(now - Duration::milliseconds(500)),
        transmit_timestamp: SntpTimestamp::from_datetime(now - Duration::milliseconds(500)),
      }))
    });

    let result = synchronize(&mut server_mock);
    assert!(result.is_err());
  }

  #[test]
  fn sync_fails_if_reply_contains_zero_transmit_timestamp() {
    let mut server_mock = ServerMock::new(|packet: &Packet| {
      let now = Utc::now();

      Ok(Ok(Packet {
        li: LeapIndicator::NoWarning,
        mode: Mode::Server,
        stratum: 1,
        reference_identifier: ReferenceIdentifier::new_ascii([0x4c, 0x4f, 0x43, 0x4c]).unwrap(),
        reference_timestamp: SntpTimestamp::from_datetime(now - Duration::days(1)),
        originate_timestamp: packet.transmit_timestamp,
        receive_timestamp: SntpTimestamp::from_datetime(now - Duration::milliseconds(500)),
        transmit_timestamp: SntpTimestamp::zero(),
      }))
    });

    let result = synchronize(&mut server_mock);
    assert!(result.is_err());
  }

  #[test]
  fn sync_fails_if_reply_contains_wrong_mode() {
    let mut server_mock = ServerMock::new(|packet: &Packet| {
      let now = Utc::now();

      Ok(Ok(Packet {
        li: LeapIndicator::NoWarning,
        mode: Mode::Client,
        stratum: 1,
        reference_identifier: ReferenceIdentifier::new_ascii([0x4c, 0x4f, 0x43, 0x4c]).unwrap(),
        reference_timestamp: SntpTimestamp::from_datetime(now - Duration::days(1)),
        originate_timestamp: packet.transmit_timestamp,
        receive_timestamp: SntpTimestamp::from_datetime(now - Duration::milliseconds(500)),
        transmit_timestamp: SntpTimestamp::from_datetime(now - Duration::milliseconds(500)),
      }))
    });

    let result = synchronize(&mut server_mock);
    assert!(result.is_err());
  }

  #[test]
  fn sync_fails_if_kiss_o_death_received() {
    let mut server_mock = ServerMock::new(|packet: &Packet| {
      let now = Utc::now();

      Ok(Ok(Packet {
        li: LeapIndicator::NoWarning,
        mode: Mode::Server,
        stratum: 0,
        reference_identifier: ReferenceIdentifier::new_ascii([0x52, 0x41, 0x54, 0x45]).unwrap(),
        reference_timestamp: SntpTimestamp::from_datetime(now - Duration::days(1)),
        originate_timestamp: packet.transmit_timestamp,
        receive_timestamp: SntpTimestamp::from_datetime(now - Duration::milliseconds(500)),
        transmit_timestamp: SntpTimestamp::from_datetime(now - Duration::milliseconds(500)),
      }))
    });

    let err = synchronize(&mut server_mock).unwrap_err();

    if let SynchroniztationError::ProtocolError(ProtocolError::KissODeath(KissCode::RateExceeded)) =
      err
    {
      // pass
    } else {
      panic!("Wrong error received");
    }
  }
}
