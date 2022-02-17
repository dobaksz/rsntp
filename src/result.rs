use crate::packet::{LeapIndicator, ReferenceIdentifier};

/// Represents a signed duration value.
///
/// It's main purpose is to store signed duration values which the [`std::time::Duration`] is not
/// capable of. It can be converted to a different duration representation, depending on the
/// enabled time crate support or it has some methods to inspect its value directly.
///
/// If you want to use it directly then you can use [`as_secs_f64`], [`abs_as_std_duration`]
/// and [`signum`] methods.
///
/// If chrono support is enabled then you can convert it to [`chrono::Duration`]
/// with [`Self::as_chrono_duration`].
#[derive(Debug, Clone, Copy, PartialEq, PartialOrd)]
pub struct SntpDuration(f64);

impl SntpDuration {
    pub(crate) fn from_secs_f64(secs: f64) -> SntpDuration {
        SntpDuration(secs)
    }

    /// Returns with the absolute value of the duration
    ///
    /// As [`std::time::Duration`] cannot store signed values, the returned duration will always be
    /// positive and will store the absolute value.
    ///
    /// ```no_run
    /// use rsntp::SntpClient;
    ///
    /// let client = SntpClient::new();
    /// let result = client.synchronize("pool.ntp.org").unwrap();
    /// let clock_offset_abs = result.clock_offset().abs_as_std_duration().as_secs_f64();
    /// let clock_offset = clock_offset_abs * result.clock_offset().signum() as f64;
    ///
    /// println!("Clock offset: {} seconds", clock_offset);
    /// ```
    pub fn abs_as_std_duration(&self) -> std::time::Duration {
        std::time::Duration::from_secs_f64(self.0.abs())
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
    /// let clock_offset_abs = result.clock_offset().abs_as_std_duration().as_secs_f64();
    /// let clock_offset = clock_offset_abs * result.clock_offset().signum() as f64;
    ///
    /// println!("Clock offset: {} seconds", clock_offset);
    /// ```
    pub fn signum(&self) -> i32 {
        self.0.signum() as i32
    }

    /// Returns with the number of seconds in this duration as a floating point number
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

    /// Converts the duration to [`chrono::Duration`]
    ///
    /// Only available when chrono crate support is enabled
    ///
    /// ```no_run
    /// use rsntp::SntpClient;
    ///
    /// let client = SntpClient::new();
    /// let result = client.synchronize("pool.ntp.org").unwrap();
    ///
    /// let clock_offset: chrono::Duration = result.clock_offset().as_chrono_duration();
    /// ```
    pub fn as_chrono_duration(&self) -> chrono::Duration {
        chrono::Duration::from_std(self.abs_as_std_duration()).unwrap() * self.signum()
    }
}

/// Represents a date and time
///
/// It's main purpose is to have an indenedent wrapper for different date and time representations.
/// It is not intended to be used directly, but should be converted to a different duration
/// representation, depending on the enabled time crate support.
///
/// If chrono support is enabled (which is the default), then it can be converted to
/// [`chrono::DateTime<Utc>´] with [`Self::as_chrono_datetime_utc`]
#[derive(Debug, Clone, Copy)]
pub struct SntpDateTime {
    offset: SntpDuration,
}

impl SntpDateTime {
    pub(crate) fn new(offset: SntpDuration) -> SntpDateTime {
        SntpDateTime { offset }
    }

    /// Converts the date and time to [`chrono::DateTime<Utc>`]
    ///
    /// Only available when chrono crate support is enabled
    ///
    /// Calcuating synchronized local time:
    /// ```no_run
    /// use rsntp::SntpClient;
    /// use chrono::{DateTime, Local};
    ///
    /// let client = SntpClient::new();
    /// let result = client.synchronize("pool.ntp.org").unwrap();
    ///
    /// let local_time: DateTime<Local> = DateTime::from(result.datetime().as_chrono_datetime_utc());
    /// ```
    pub fn as_chrono_datetime_utc(&self) -> chrono::DateTime<chrono::Utc> {
        chrono::Utc::now() + self.offset.as_chrono_duration()
    }
}

/// Results of a synchronization.
///
/// If you just simply need a fairly accurate SNTP time then check the `datetime()` method. Other methods
/// provide more detailed information about the outcome of the synchronization and might need deeper
/// knwoledge about  SNTP protocol internals.
#[derive(Debug, Clone)]
pub struct SynchronizationResult {
    clock_offset_s: f64,
    round_trip_delay_s: f64,
    reference_identifier: ReferenceIdentifier,
    leap_indicator: LeapIndicator,
    stratum: u8,
}

impl SynchronizationResult {
    pub(crate) fn new(
        clock_offset_s: f64,
        round_trip_delay_s: f64,
        reference_identifier: ReferenceIdentifier,
        leap_indicator: LeapIndicator,
        stratum: u8,
    ) -> SynchronizationResult {
        SynchronizationResult {
            clock_offset_s,
            round_trip_delay_s,
            reference_identifier,
            leap_indicator,
            stratum,
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
    /// use chrono::Local;
    ///
    /// let client = SntpClient::new();
    /// let result = client.synchronize("pool.ntp.org").unwrap();
    ///
    /// println!("Local time: {}", Local::now() + result.clock_offset().as_chrono_duration());
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
    /// use chrono::Local;
    ///
    /// let client = SntpClient::new();
    /// let result = client.synchronize("pool.ntp.org").unwrap();
    ///
    /// println!("RTT: {} ms", result.round_trip_delay().as_chrono_duration().num_milliseconds());
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
    /// use chrono::Local;
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
    /// This is the current UTC date and time, calculated by adding clock offset the UTC time. To be accurate,
    /// use the returned value immediately.
    ///
    /// # Example
    ///
    /// Calcuating synchronized local time:
    /// ```no_run
    /// use rsntp::SntpClient;
    /// use chrono::{DateTime, Local};
    ///
    /// let client = SntpClient::new();
    /// let result = client.synchronize("pool.ntp.org").unwrap();
    ///
    /// let local_time: DateTime<Local> = DateTime::from(result.datetime().as_chrono_datetime_utc());
    /// ```
    pub fn datetime(&self) -> SntpDateTime {
        SntpDateTime::new(self.clock_offset())
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
    /// use chrono::{DateTime, Local};
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
            positive_duration.abs_as_std_duration(),
            std::time::Duration::from_secs(3600)
        );
        assert_eq!(
            negative_duration.abs_as_std_duration(),
            std::time::Duration::from_secs(3600)
        );

        assert_eq!(positive_duration.signum(), 1);
        assert_eq!(negative_duration.signum(), -1);
    }

    #[test]
    fn sntp_duration_converting_to_chrono_duration_works() {
        let positive_duration = SntpDuration::from_secs_f64(3600.0);
        let negative_duration = SntpDuration::from_secs_f64(-3600.0);

        assert_eq!(
            positive_duration.as_chrono_duration(),
            chrono::Duration::hours(1)
        );
        assert_eq!(
            negative_duration.as_chrono_duration(),
            chrono::Duration::hours(-1)
        );
    }

    #[test]
    fn sntp_date_time_converting_to_chrono_datetime_works() {
        let datetime = SntpDateTime::new(SntpDuration::from_secs_f64(0.1));
        let converted = datetime.as_chrono_datetime_utc();
        let diff = converted - chrono::Utc::now();

        assert!(diff.num_milliseconds() > 90);
        assert!(diff.num_milliseconds() < 110);
    }
}
