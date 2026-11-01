#[cfg(feature = "bigdecimal-04")]
mod bigdecimal;
#[cfg(feature = "chrono-04")]
mod chrono;
#[cfg(feature = "jiff-02")]
mod jiff;
#[cfg(feature = "rust_decimal-1")]
mod rust_decimal;
#[cfg(feature = "time-03")]
mod time;

/// The coarsest of seconds, millis, micros and nanos that holds `nanos` exactly.
#[cfg(any(feature = "chrono-04", feature = "jiff-02", feature = "time-03"))]
fn exact_precision(nanos: i64) -> crate::TimePrecision {
    use crate::TimePrecision;

    [
        TimePrecision::SECONDS,
        TimePrecision::MILLIS,
        TimePrecision::MICROS,
    ]
    .into_iter()
    .find(|precision| nanos % precision.unit_nanos() == 0)
    .unwrap_or(TimePrecision::NANOS)
}
