# Changlelog

## 4.0.0
- Correct typo in SynchronizationError (might break compilation)
- Helper function to check for Kiss-o'-death
- SntpDateTime is now convertible to SystemTime

## 3.0.2
- Added Default trait implementation for Config struct
- Documentation corrections

## 3.0.1
- Minor documentation corrections

## 3.0.0
- Core code made idependent of `chrono` crate, `chrono` feature is added to disable support for that crate
- Added support for returning results in `time` crate format

## 2.1.0
- New configuration API, which allows to set instance config without making it mutable

## 2.0.0
- Use tokio 1.0

## 1.0.3
- Proper IPv6 release
- Small documentation improvements

## 1.0.2
- ToServerAddrs is implmented for (String, u16)
- Fixed error handling cases related to socket handling

## 1.0.1
- Minor fix,corrected README

## 1.0.0
- New unified API with extending addressing support
- Methods to set default bind address

## 0.3.2
- Fix under/overflow on timestamps before 1970

## 0.3.1
- Fixed versions in documentation

## 0.3.0
- Method to set timeout, small fixes

## 0.2.0
- Added optional asynchronous API

## 0.1.0
- Initial release