use crate::clock::Clock;
use crate::error::ConversionError;
use crate::packet::{LeapIndicator, ReferenceIdentifier};
#[cfg(all(feature = "chrono", feature = "time"))]
use std::convert::TryInto;
use std::time::SystemTime;

/// Represents a signed duration value.
///
/// It's main purpose is to store signed duration values which the [`std::time::Duration`] is not
/// capable of, while making it possible to return a time-crate independent duration values
/// (i.e. it works without `chrono` support enabled).
///
/// It can be converted to a different duration representation, depending on the
/// enabled time crate support or it has some methods to inspect its value directly.
///
/// If `chrono` crate support is enabled then it will have [`TryInto<chrono::Duration>`] implemented.
/// If `time` crate support is enabled then it will have [`TryInto<time::Duration>`] implemented.
#[derive(Debug, Clone, Copy, PartialEq, PartialOrd)]
pub struct SntpDuration(f64);

impl SntpDuration {
    pub(crate) fn from_secs_f64(secs: f64) -> SntpDuration {
        SntpDuration(secs)
    }

    /// Returns with the absolute value of the duration
    ///
    /// As [`std::time::Duration`] cannot store signed values, the returned duration will always be
    /// positive and will store the absolute value. This is a fallible conversion as there can be cases
    /// when duration contains a non-convertible number.
    ///
    /// ```no_run
    /// use rsntp::SntpClient;
    ///
    /// let client = SntpClient::new();
    /// let result = client.synchronize("pool.ntp.org").unwrap();
    /// let clock_offset_abs = result.clock_offset().abs_as_std_duration().unwrap().as_secs_f64();
    /// let clock_offset = clock_offset_abs * result.clock_offset().signum() as f64;
    ///
    /// println!("Clock offset: {} seconds", clock_offset);
    /// ```
    pub fn abs_as_std_duration(&self) -> Result<std::time::Duration, ConversionError> {
        std::time::Duration::try_from_secs_f64(self.0.abs()).map_err(|_| ConversionError::Overflow)
    }

    /// Returns with the sign of the duration
    ///
    /// Works similar way as `signum` methods for built-in types, returns with `1` if the
    /// duration is positive or with `-1` if the duration is negative.
    ///
    /// ```no_run
    /// use rsntp::SntpClient;
    ///
    /// let client = SntpClient::new();
    /// let result = client.synchronize("pool.ntp.org").unwrap();
    /// let clock_offset_abs = result.clock_offset().abs_as_std_duration().unwrap().as_secs_f64();
    /// let clock_offset = clock_offset_abs * result.clock_offset().signum() as f64;
    ///
    /// println!("Clock offset: {} seconds", clock_offset);
    /// ```
    pub fn signum(&self) -> i32 {
        self.0.signum() as i32
    }

    /// Returns with the number of seconds in this duration as a floating point number
    ///
    /// The returned value will have a proper sign, i.e. it will be negative if the
    /// stored duration is negative.
    ///
    /// ```no_run
    /// use rsntp::SntpClient;
    ///
    /// let client = SntpClient::new();
    /// let result = client.synchronize("pool.ntp.org").unwrap();
    ///
    /// println!("Clock offset: {} seconds", result.clock_offset().as_secs_f64());
    /// ```
    pub fn as_secs_f64(&self) -> f64 {
        self.0
    }

    /// Convert instance to [`chrono::Duration`]
    ///
    /// Convenience wrapper for [`TryInto<chrono::Duration>::try_into`] to avoid
    /// type annotations.
    #[cfg(feature = "chrono")]
    pub fn into_chrono_duration(self) -> Result<chrono::Duration, ConversionError> {
        self.try_into()
    }

    /// Convert instance to [`time::Duration`]
    ///
    /// Convenience wrapper for [`TryInto<time::Duration>::try_into`] to avoid
    /// type annotations.
    #[cfg(feature = "time")]
    pub fn into_time_duration(self) -> Result<time::Duration, ConversionError> {
        self.try_into()
    }
}

#[cfg(feature = "chrono")]
impl TryInto<chrono::Duration> for SntpDuration {
    type Error = ConversionError;

    fn try_into(self) -> Result<chrono::Duration, ConversionError> {
        let abs = chrono::Duration::from_std(self.abs_as_std_duration()?)
            .map_err(|_| ConversionError::Overflow)?;

        Ok(abs * self.signum())
    }
}

#[cfg(feature = "time")]
impl TryInto<time::Duration> for SntpDuration {
    type Error = ConversionError;

    fn try_into(self) -> Result<time::Duration, ConversionError> {
        Ok(time::Duration::seconds_f64(self.0))
    }
}

/// Represents a date and time
///
/// It's main purpose is to have a wrapper for different date and time representations, which
/// is usable regadless of the enabled time crate support.
///
/// It can be inspected directly, but there is no built-in timezone conversion, it will
/// always return with UTC timestamps. If you need timezone support then you have to use
/// `chrono` or `time` crate for conversion.
///
/// The conversions return with the current synchronized time at the moment of the conversion, i.e.
/// converting the same instance later returns with a later time. The elapsed time since the
/// synchronization is measured with the steady clock, so system clock changes do not affect it.
///
/// If `chrono` crate support is enabled then it will have [`TryInto<chrono::DateTime<Utc>>`] implemented.
/// If `time` crate support is enabled then it will have [`TryInto<time::OffsetDateTime>`] implemented.
#[derive(Clone, Copy)]
pub struct SntpDateTime {
    clock: Clock,
    offset: SntpDuration,
}

// The clock is an internal detail, so it is left out of the debug output
impl std::fmt::Debug for SntpDateTime {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("SntpDateTime")
            .field("offset", &self.offset)
            .finish()
    }
}

impl SntpDateTime {
    pub(crate) fn new(clock: Clock, offset: SntpDuration) -> SntpDateTime {
        SntpDateTime { clock, offset }
    }

    fn corrected(&self) -> Result<SystemTime, ConversionError> {
        let local_now = self.clock.now();
        let offset_abs = self.offset.abs_as_std_duration()?;

        if self.offset.signum() >= 0 {
            local_now.checked_add(offset_abs)
        } else {
            local_now.checked_sub(offset_abs)
        }
        .ok_or(ConversionError::Overflow)
    }

    /// Returns with the duration since Unix epoch i.e. Unix timestamp
    ///
    /// Then conversion can fail in cases like internal overflow or when
    /// the date is not representable with a Unix timestamp (like it is
    /// before Unix epoch).
    ///
    /// ```no_run
    /// use rsntp::SntpClient;
    ///
    /// let client = SntpClient::new();
    /// let result = client.synchronize("pool.ntp.org").unwrap();
    ///
    /// let unix_timetamp_utc = result.datetime().unix_timestamp().unwrap();
    /// ```
    pub fn unix_timestamp(&self) -> Result<std::time::Duration, ConversionError> {
        self.corrected()?
            .duration_since(SystemTime::UNIX_EPOCH)
            .map_err(|_| ConversionError::Overflow)
    }

    /// Convert instance to [`std::time::SystemTime`].
    ///
    /// Convenience wrapper for [`TryInto<std::time::SystemTime>::try_into`]
    /// to avoid type annotations.
    pub fn into_system_time(self) -> Result<std::time::SystemTime, ConversionError> {
        self.try_into()
    }

    /// Convert instance to [`chrono::DateTime<chrono::Utc>`].
    ///
    /// Convenience wrapper for [`TryInto<chrono::DateTime<chrono::Utc>>::try_into`]
    /// to avoid type annotations.
    #[cfg(feature = "chrono")]
    pub fn into_chrono_datetime(self) -> Result<chrono::DateTime<chrono::Utc>, ConversionError> {
        self.try_into()
    }

    /// Convert instance to [`time::OffsetDateTime`].
    ///
    /// Convenience wrapper for [`TryInto<time::OffsetDateTime>::try_into`]
    /// to avoid type annotations.
    #[cfg(feature = "time")]
    pub fn into_offset_date_time(self) -> Result<time::OffsetDateTime, ConversionError> {
        self.try_into()
    }
}

impl TryInto<std::time::SystemTime> for SntpDateTime {
    type Error = ConversionError;

    fn try_into(self) -> Result<std::time::SystemTime, ConversionError> {
        self.corrected()
    }
}

#[cfg(feature = "chrono")]
impl TryInto<chrono::DateTime<chrono::Utc>> for SntpDateTime {
    type Error = ConversionError;

    fn try_into(self) -> Result<chrono::DateTime<chrono::Utc>, ConversionError> {
        let chrono_offset: chrono::Duration = self.offset.try_into()?;

        chrono::DateTime::<chrono::Utc>::from(self.clock.now())
            .checked_add_signed(chrono_offset)
            .ok_or(ConversionError::Overflow)
    }
}

#[cfg(feature = "time")]
impl TryInto<time::OffsetDateTime> for SntpDateTime {
    type Error = ConversionError;

    fn try_into(self) -> Result<time::OffsetDateTime, ConversionError> {
        let time_offset: time::Duration = self.offset.try_into()?;

        time::OffsetDateTime::from(self.clock.now())
            .checked_add(time_offset)
            .ok_or(ConversionError::Overflow)
    }
}

/// Results of a synchronization.
///
/// If you just simply need a fairly accurate SNTP time then check the `datetime()` method. Other methods
/// provide more detailed information about the outcome of the synchronization and might need deeper
/// knwoledge about  SNTP protocol internals.
#[derive(Clone)]
pub struct SynchronizationResult {
    clock_offset_s: f64,
    round_trip_delay_s: f64,
    reference_identifier: ReferenceIdentifier,
    leap_indicator: LeapIndicator,
    stratum: u8,
    clock: Clock,
}

// The clock is an internal detail, so it is left out of the debug output
impl std::fmt::Debug for SynchronizationResult {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("SynchronizationResult")
            .field("clock_offset_s", &self.clock_offset_s)
            .field("round_trip_delay_s", &self.round_trip_delay_s)
            .field("reference_identifier", &self.reference_identifier)
            .field("leap_indicator", &self.leap_indicator)
            .field("stratum", &self.stratum)
            .finish()
    }
}

impl SynchronizationResult {
    pub(crate) fn new(
        clock_offset_s: f64,
        round_trip_delay_s: f64,
        reference_identifier: ReferenceIdentifier,
        leap_indicator: LeapIndicator,
        stratum: u8,
        clock: Clock,
    ) -> SynchronizationResult {
        SynchronizationResult {
            clock_offset_s,
            round_trip_delay_s,
            reference_identifier,
            leap_indicator,
            stratum,
            clock,
        }
    }

    /// Returns with the offset between server and local clock.
    ///
    /// It is a signed duration, negative value means the local clock is ahead.
    ///
    /// # Example
    ///
    /// Print the synchronized local time using clock offset:
    /// ```no_run
    /// use rsntp::SntpClient;
    ///
    /// let client = SntpClient::new();
    /// let result = client.synchronize("pool.ntp.org").unwrap();
    ///
    /// println!("Clock offset: {}", result.clock_offset().as_secs_f64());
    /// ```
    pub fn clock_offset(&self) -> SntpDuration {
        SntpDuration::from_secs_f64(self.clock_offset_s)
    }

    /// Returns with the round trip delay
    ///
    /// The time is needed for SNTP packets to travel back and forth between the host and the server.
    /// It is a signed value but negative values should not be possible in client mode
    /// (which is currently always used by the library).
    ///
    /// # Example
    ///
    /// ```no_run
    /// use rsntp::SntpClient;
    ///
    /// let client = SntpClient::new();
    /// let result = client.synchronize("pool.ntp.org").unwrap();
    ///
    /// println!("RTT: {} ms", result.round_trip_delay().as_secs_f64() * 1000.0);
    /// ```
    pub fn round_trip_delay(&self) -> SntpDuration {
        SntpDuration::from_secs_f64(self.round_trip_delay_s)
    }

    /// Returns with the server reference identifier.
    ///
    /// This identifies the synchronizaion source of the server. For primary servers (startum = 1) this is a four
    /// byte ASCII string, for secondary IPv4 servers (startum >= 2) this is an IP address, for secondary IPv6
    /// servers this contains first 32 bits of an MD5 hash of an IPv6 address.
    ///   
    /// # Example
    ///
    /// ```no_run
    /// use rsntp::SntpClient;
    ///
    /// let client = SntpClient::new();
    /// let result = client.synchronize("pool.ntp.org").unwrap();
    ///
    /// println!("Server reference identifier: {}", result.reference_identifier());
    /// ```
    pub fn reference_identifier(&self) -> &ReferenceIdentifier {
        &self.reference_identifier
    }

    /// Returns with the current UTC date and time, based on the synchronized SNTP timestamp.
    ///
    /// This is the current UTC date and time, calculated by adding the clock offset to the local time.
    /// The local time is the time of the synchronization plus the time elapsed since then, measured
    /// with the steady clock, so changes to the system clock after synchronization do not affect it.
    ///
    /// The returned value calculates the current time when it is converted, see [`SntpDateTime`].
    ///
    /// # Example
    ///
    /// Calcuating synchronized local time:
    /// ```no_run
    /// use rsntp::SntpClient;
    ///
    /// let client = SntpClient::new();
    /// let result = client.synchronize("pool.ntp.org").unwrap();
    ///
    /// let unix_timetamp_utc = result.datetime().unix_timestamp().unwrap();
    /// ```
    pub fn datetime(&self) -> SntpDateTime {
        SntpDateTime::new(self.clock, self.clock_offset())
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
    ///
    /// let client = SntpClient::new();
    /// let result = client.synchronize("pool.ntp.org").unwrap();
    ///
    /// println!("Leap indicator: {:?}", result.leap_indicator());
    /// ```
    pub fn leap_indicator(&self) -> LeapIndicator {
        self.leap_indicator
    }

    /// Returns with the server stratum
    ///
    /// NTP uses a hierarchical, semi-layered system of time sources. Each level of this hierarchy is
    /// termed a stratum and is assigned a number starting with zero for the reference clock at the top.
    /// A server synchronized to a stratum n server runs at stratum n + 1
    ///
    /// Values defined as:
    /// *  1 - Primary reference (e.g., calibrated atomic clock, radio clock, etc...)
    /// *  2..15 - Secondary reference (via NTP, calculated as the stratum of system peer plus one)
    /// *  16 - Unsynchronized
    /// *  16..255 - Reserved
    ///
    /// # Example
    ///
    /// ```no_run
    /// use rsntp::SntpClient;
    ///
    /// let client = SntpClient::new();
    /// let result = client.synchronize("pool.ntp.org").unwrap();
    ///
    /// assert!(result.stratum() >= 1);
    /// ```
    pub fn stratum(&self) -> u8 {
        self.stratum
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::{Duration, Instant};

    #[test]
    fn sntp_duration_as_secs_f64_works() {
        let positive_duration = SntpDuration::from_secs_f64(3600.0);
        let negative_duration = SntpDuration::from_secs_f64(-3600.0);

        assert_eq!(positive_duration.as_secs_f64(), 3600.0);
        assert_eq!(negative_duration.as_secs_f64(), -3600.0);
    }

    #[test]
    fn sntp_duration_abs_and_signum_works() {
        let positive_duration = SntpDuration::from_secs_f64(3600.0);
        let negative_duration = SntpDuration::from_secs_f64(-3600.0);

        assert_eq!(
            positive_duration.abs_as_std_duration().unwrap(),
            std::time::Duration::from_secs(3600)
        );
        assert_eq!(
            negative_duration.abs_as_std_duration().unwrap(),
            std::time::Duration::from_secs(3600)
        );

        assert_eq!(positive_duration.signum(), 1);
        assert_eq!(negative_duration.signum(), -1);
    }

    #[test]
    fn sntp_duration_abs_fails_on_overflow() {
        let duration = SntpDuration::from_secs_f64(2e19);

        assert!(duration.abs_as_std_duration().is_err());
    }

    #[cfg(feature = "chrono")]
    #[test]
    fn sntp_duration_converting_to_chrono_duration_works() {
        let positive_duration = SntpDuration::from_secs_f64(3600.0);
        let negative_duration = SntpDuration::from_secs_f64(-3600.0);

        let positive_chrono: chrono::Duration = positive_duration.try_into().unwrap();
        let negative_chrono: chrono::Duration = negative_duration.try_into().unwrap();

        assert_eq!(positive_chrono, chrono::Duration::hours(1));
        assert_eq!(negative_chrono, chrono::Duration::hours(-1));
    }

    #[cfg(feature = "chrono")]
    #[test]
    fn sntp_duration_converting_to_chrono_duration_fails() {
        let nan_duration_result: Result<chrono::Duration, ConversionError> =
            SntpDuration::from_secs_f64(f64::NAN).try_into();

        assert!(nan_duration_result.is_err());
    }

    #[cfg(feature = "time")]
    #[test]
    fn sntp_duration_converting_to_time_duration_works() {
        let positive_duration = SntpDuration::from_secs_f64(3600.0);
        let negative_duration = SntpDuration::from_secs_f64(-3600.0);

        let positive_time: time::Duration = positive_duration.try_into().unwrap();
        let negative_time: time::Duration = negative_duration.try_into().unwrap();

        assert_eq!(positive_time, time::Duration::hours(1));
        assert_eq!(negative_time, time::Duration::hours(-1));
    }

    #[test]
    fn sntp_date_time_unix_timestamp_works() {
        let system_time = SystemTime::UNIX_EPOCH + Duration::from_secs(1_000_000);
        let steady_time = Instant::now();
        let clock = Clock::new(system_time, steady_time);
        let positive = SntpDateTime::new(clock, SntpDuration::from_secs_f64(5.0));
        let negative = SntpDateTime::new(clock, SntpDuration::from_secs_f64(-5.0));

        let positive_timestamp = positive.unix_timestamp().unwrap();
        let negative_timestamp = negative.unix_timestamp().unwrap();
        let elapsed = steady_time.elapsed();

        assert!(positive_timestamp >= Duration::from_secs(1_000_005));
        assert!(positive_timestamp <= Duration::from_secs(1_000_005) + elapsed);
        assert!(negative_timestamp >= Duration::from_secs(999_995));
        assert!(negative_timestamp <= Duration::from_secs(999_995) + elapsed);
    }

    #[test]
    fn sntp_date_time_unix_timestamp_fails_before_epoch() {
        let system_time = SystemTime::UNIX_EPOCH + Duration::from_secs(1_000_000);
        let clock = Clock::new(system_time, Instant::now());
        let datetime = SntpDateTime::new(clock, SntpDuration::from_secs_f64(-2_000_000.0));

        assert!(datetime.unix_timestamp().is_err());
    }

    #[test]
    fn sntp_date_time_converting_to_system_time_works() {
        let system_time = SystemTime::UNIX_EPOCH + Duration::from_secs(1_000_000);
        let steady_time = Instant::now();
        let clock = Clock::new(system_time, steady_time);
        let positive = SntpDateTime::new(clock, SntpDuration::from_secs_f64(3600.0));
        let negative = SntpDateTime::new(clock, SntpDuration::from_secs_f64(-3600.0));

        let positive_converted = positive.into_system_time().unwrap();
        let negative_converted = negative.into_system_time().unwrap();
        let elapsed = steady_time.elapsed();

        let positive_expected = system_time + Duration::from_secs(3600);
        let negative_expected = system_time - Duration::from_secs(3600);
        assert!(positive_converted >= positive_expected);
        assert!(positive_converted <= positive_expected + elapsed);
        assert!(negative_converted >= negative_expected);
        assert!(negative_converted <= negative_expected + elapsed);
    }

    #[test]
    fn sntp_date_time_converting_to_system_time_fails_for_nan() {
        let clock = Clock::new(SystemTime::UNIX_EPOCH, Instant::now());
        let datetime = SntpDateTime::new(clock, SntpDuration::from_secs_f64(f64::NAN));

        assert!(datetime.into_system_time().is_err());
    }

    #[test]
    fn sntp_date_time_follows_steady_clock() {
        let system_time = SystemTime::UNIX_EPOCH + Duration::from_secs(1_000_000);
        let steady_time = Instant::now().checked_sub(Duration::from_secs(10)).unwrap();
        let clock = Clock::new(system_time, steady_time);
        let datetime = SntpDateTime::new(clock, SntpDuration::from_secs_f64(5.0));

        let timestamp = datetime.unix_timestamp().unwrap();
        let elapsed = steady_time.elapsed();

        assert!(timestamp >= Duration::from_secs(1_000_015));
        assert!(timestamp <= Duration::from_secs(1_000_005) + elapsed);
    }

    #[test]
    fn sntp_date_time_advances_between_conversions() {
        let clock = Clock::new(SystemTime::UNIX_EPOCH, Instant::now());
        let datetime = SntpDateTime::new(clock, SntpDuration::from_secs_f64(0.0));

        let converted_1 = datetime.into_system_time().unwrap();
        std::thread::sleep(Duration::from_millis(10));
        let converted_2 = datetime.into_system_time().unwrap();

        assert!(converted_2.duration_since(converted_1).unwrap() >= Duration::from_millis(10));
    }

    #[cfg(feature = "chrono")]
    #[test]
    fn sntp_date_time_converting_to_chrono_datetime_works() {
        let system_time = SystemTime::UNIX_EPOCH + Duration::from_secs(1_000_000);
        let steady_time = Instant::now();
        let clock = Clock::new(system_time, steady_time);
        let datetime = SntpDateTime::new(clock, SntpDuration::from_secs_f64(0.1));

        let converted: chrono::DateTime<chrono::Utc> = datetime.try_into().unwrap();
        let elapsed = steady_time.elapsed();

        let converted = SystemTime::from(converted);
        let expected = system_time + Duration::from_millis(100);
        assert!(converted >= expected);
        assert!(converted <= expected + elapsed);
    }

    #[cfg(feature = "chrono")]
    #[test]
    fn sntp_date_time_converting_to_chrono_datetime_fails_for_nan() {
        let clock = Clock::new(SystemTime::UNIX_EPOCH, Instant::now());
        let datetime = SntpDateTime::new(clock, SntpDuration::from_secs_f64(f64::NAN));

        let converted: Result<chrono::DateTime<chrono::Utc>, ConversionError> = datetime.try_into();

        assert!(converted.is_err());
    }

    #[cfg(feature = "time")]
    #[test]
    fn sntp_date_time_converting_to_time_offset_datetime_works() {
        let system_time = SystemTime::UNIX_EPOCH + Duration::from_secs(1_000_000);
        let steady_time = Instant::now();
        let clock = Clock::new(system_time, steady_time);
        let datetime = SntpDateTime::new(clock, SntpDuration::from_secs_f64(-0.1));

        let converted: time::OffsetDateTime = datetime.try_into().unwrap();
        let elapsed = steady_time.elapsed();

        let converted = SystemTime::from(converted);
        let expected = system_time - Duration::from_millis(100);
        assert!(converted >= expected);
        assert!(converted <= expected + elapsed);
    }

    #[test]
    fn synchronization_result_datetime_follows_steady_clock() {
        let system_time = SystemTime::UNIX_EPOCH + Duration::from_secs(1_000_000);
        let steady_time = Instant::now().checked_sub(Duration::from_secs(10)).unwrap();
        let result = SynchronizationResult::new(
            5.0,
            0.1,
            ReferenceIdentifier::Empty,
            LeapIndicator::NoWarning,
            1,
            Clock::new(system_time, steady_time),
        );

        let timestamp = result.datetime().unix_timestamp().unwrap();
        let elapsed = steady_time.elapsed();

        assert!(timestamp >= Duration::from_secs(1_000_015));
        assert!(timestamp <= Duration::from_secs(1_000_005) + elapsed);
    }
}
