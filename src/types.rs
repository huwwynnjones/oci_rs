use byteorder::{BigEndian, ByteOrder, LittleEndian};
use chrono::{Date, DateTime, Datelike, FixedOffset, TimeZone, Timelike, Utc};
use libc::{c_int, c_void};
use oci_bindings::OciDataType;
use oci_error::OciError;

/// The types that support conversion from OCI to Rust types.
///
#[derive(Debug, PartialEq, Clone)]
pub enum SqlValue {
    /// Anything specified as `VARCHAR` or `VARCHAR2` will end up here.
    VarChar(String),
    /// Represents `CHAR`
    Char(String),
    /// All integers regardless of their stated size are represented with this variant. e.g.
    /// `SMALLINT` and `INTEGER` will both be held.
    Integer(i64),
    /// All floating point types regardless of their size are represented with this variant. e.g.
    /// `REAL` and `FLOAT` will both be held.
    Float(f64),
    /// Represents null values in columns.
    Null,
    /// Represents a date
    Date(Date<Utc>, [u8; 7]),
    /// Represents a timestamp without time zone
    Timestamp(DateTime<Utc>, [u8; 11]),
    /// Represents a timestamp with a time zone
    TimestampTz(DateTime<FixedOffset>, [u8; 13]),
}
impl SqlValue {
    /// Returns the internal value converting on the way to whichever type implements
    /// `FromSqlValue`.
    ///
    /// It returns an `Option` because conversion might not be possible.
    /// For example converting an `SqlValue::Integer` to a `String` works just fine, but converting
    /// an `SqlValue::Null` to an i64 does not make sense.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use oci_rs::types::{SqlValue, ToSqlValue};
    ///
    /// let v = SqlValue::Integer(42);
    /// let i: i64 = v.value().expect("Won't convert to an i64");
    /// let s: String = v.value().expect("Won't convert to a String");
    ///
    /// assert_eq!(i, 42);
    /// assert_eq!(s, "42");
    ///
    /// let null = SqlValue::Null;
    /// let null_as_i64: Option<i64> = null.value();
    ///
    /// assert_eq!(null_as_i64, None);
    /// ```
    ///
    pub fn value<T: FromSqlValue>(&self) -> Option<T> {
        T::from_sql_value(self)
    }

    /// Returns a pointer to the internal value that can be used by OCI.
    ///
    pub(crate) fn as_oci_ptr(&mut self) -> *mut c_void {
        match *self {
            SqlValue::VarChar(ref s) | SqlValue::Char(ref s) => s.as_ptr() as *mut c_void,
            SqlValue::Integer(ref mut i) => (i as *mut i64) as *mut c_void,
            SqlValue::Float(ref mut f) => (f as *mut f64) as *mut c_void,
            SqlValue::Null => panic!("Null not handled"),
            SqlValue::Date(_, ref b) => b.as_ptr() as *mut c_void,
            SqlValue::Timestamp(_, ref b) => b.as_ptr() as *mut c_void,
            SqlValue::TimestampTz(_, ref b) => b.as_ptr() as *mut c_void,
        }
    }

    /// Gives the size in bytes of the internal value.
    ///
    /// It is used by the OCI library to allocate storage. Byte size values
    /// are hard coded here on purpose as a confirmation of OCI spec.
    ///
    pub(crate) fn size(&self) -> c_int {
        match *self {
            SqlValue::VarChar(ref s) | SqlValue::Char(ref s) => s.capacity() as c_int,
            SqlValue::Integer(..) | SqlValue::Float(..) => 8 as c_int,
            SqlValue::Null => panic!("Null not handled"),
            SqlValue::Date(..) => 7 as c_int,
            SqlValue::Timestamp(..) => 11 as c_int,
            SqlValue::TimestampTz(..) => 13 as c_int,
        }
    }

    /// Converts to the relevant OCI internal type.
    ///
    /// Date is converted into characters before sending into OCI
    /// this avoids having to convert a rust date object into the Oracle
    /// seven byte date format.
    ///
    pub(crate) fn as_oci_data_type(&self) -> OciDataType {
        match *self {
            SqlValue::VarChar(..) => OciDataType::SqlVarChar,
            SqlValue::Char(..) => OciDataType::SqlChar,
            SqlValue::Integer(..) => OciDataType::SqlInt,
            SqlValue::Float(..) => OciDataType::SqlFloat,
            SqlValue::Null => panic!("Null not handled"),
            SqlValue::Date(..) => OciDataType::SqlDate,
            SqlValue::Timestamp(..) => OciDataType::SqlTimestamp,
            SqlValue::TimestampTz(..) => OciDataType::SqlTimestampTz,
        }
    }

    /// Create an `SqlValue` from a slice of bytes and indication of the data type
    ///
    pub(crate) fn create_from_raw(data: &[u8], sql_type: &OciDataType) -> Result<Self, OciError> {
        match *sql_type {
            OciDataType::SqlVarChar => match String::from_utf8(Vec::from(data)) {
                Ok(s) => Ok(SqlValue::VarChar(s.trim().to_string())),
                Err(err) => Err(OciError::Conversion(Box::new(err))),
            },
            OciDataType::SqlChar => match String::from_utf8(Vec::from(data)) {
                Ok(s) => Ok(SqlValue::Char(s.to_string())),
                Err(err) => Err(OciError::Conversion(Box::new(err))),
            },
            OciDataType::SqlInt => {
                let i = LittleEndian::read_i64(data);
                Ok(SqlValue::Integer(i as i64))
            }
            OciDataType::SqlFloat => {
                let f = LittleEndian::read_f64(data);
                Ok(SqlValue::Float(f as f64))
            }
            OciDataType::SqlDate => {
                let datetime = create_datetime_from_raw(data);
                let date = datetime.date();
                Ok(SqlValue::Date(date, create_raw_from_date(date)))
            }
            OciDataType::SqlTimestamp => {
                let datetime = create_datetime_from_raw(data);
                Ok(SqlValue::Timestamp(
                    datetime,
                    create_raw_from_datetime(&datetime),
                ))
            }
            OciDataType::SqlTimestampTz => {
                let datetime_tz = create_datetime_with_timezone_from_raw(data);
                Ok(SqlValue::TimestampTz(
                    datetime_tz,
                    create_raw_from_datetime_with_timezone(&datetime_tz),
                ))
            }
            ref x => panic!(format!(
                "Creating a SqlValue from raw bytes is not implemented yet for: \
                 {:?}",
                x
            )),
        }
    }
}

/// Allows conversion into a `SqlValue`.
///
pub trait ToSqlValue {
    /// Converts into a `SqlValue`.
    ///
    fn to_sql_value(&self) -> SqlValue;
}

impl ToSqlValue for String {
    fn to_sql_value(&self) -> SqlValue {
        SqlValue::VarChar(self.clone())
    }
}

impl<'a> ToSqlValue for &'a str {
    fn to_sql_value(&self) -> SqlValue {
        let s = String::from(*self);
        SqlValue::VarChar(s)
    }
}

impl ToSqlValue for i64 {
    fn to_sql_value(&self) -> SqlValue {
        SqlValue::Integer(*self)
    }
}

impl ToSqlValue for f64 {
    fn to_sql_value(&self) -> SqlValue {
        SqlValue::Float(*self)
    }
}

impl ToSqlValue for Date<Utc> {
    fn to_sql_value(&self) -> SqlValue {
        SqlValue::Date(*self, create_raw_from_date(*self))
    }
}

impl ToSqlValue for DateTime<Utc> {
    fn to_sql_value(&self) -> SqlValue {
        SqlValue::Timestamp(*self, create_raw_from_datetime(self))
    }
}

impl ToSqlValue for DateTime<FixedOffset> {
    fn to_sql_value(&self) -> SqlValue {
        SqlValue::TimestampTz(*self, create_raw_from_datetime_with_timezone(self))
    }
}

/// Allows conversion from a `SqlValue`.
///
pub trait FromSqlValue {
    /// Allows conversion from a `SqlValue`.
    ///
    /// It allows for impossible conversions though the use of `Option`.
    /// e.g. an `SqlValue::Null` cannot be converted into a i64.
    ///
    /// When the `TryFrom` trait becomes stable then this crate will probably switch to that
    /// instead.
    ///
    fn from_sql_value(sql_value: &SqlValue) -> Option<Self>
    where
        Self: Sized;
}

impl FromSqlValue for String {
    // Converts from a `SqlValue` into a `String`
    //
    // Worth noting that this is intended to convert all types into a
    // `String` representation of the value. It also does this for
    // `SqlValue::Null` for which it returns "null". That might prove a bad idea.
    //
    fn from_sql_value(sql_value: &SqlValue) -> Option<Self> {
        match *sql_value {
            SqlValue::VarChar(ref s) | SqlValue::Char(ref s) => Some(s.to_string()),
            SqlValue::Integer(i) => Some(format!("{}", i)),
            SqlValue::Float(f) => Some(format!("{}", f)),
            SqlValue::Null => Some("null".to_string()),
            SqlValue::Date(ref d, _) => Some(format!("{}", d)),
            SqlValue::Timestamp(ref d, _) => Some(format!("{}", d)),
            SqlValue::TimestampTz(ref d, _) => Some(format!("{}", d)),
        }
    }
}

impl FromSqlValue for i64 {
    fn from_sql_value(sql_value: &SqlValue) -> Option<Self> {
        match *sql_value {
            SqlValue::Integer(i) => Some(i),
            _ => None,
        }
    }
}

impl FromSqlValue for f64 {
    fn from_sql_value(sql_value: &SqlValue) -> Option<Self> {
        match *sql_value {
            SqlValue::Float(f) => Some(f),
            _ => None,
        }
    }
}

impl FromSqlValue for Date<Utc> {
    fn from_sql_value(sql_value: &SqlValue) -> Option<Self> {
        match *sql_value {
            SqlValue::Date(d, _) => Some(d),
            _ => None,
        }
    }
}

impl FromSqlValue for DateTime<Utc> {
    fn from_sql_value(sql_value: &SqlValue) -> Option<Self> {
        match *sql_value {
            SqlValue::Timestamp(d, _) => Some(d),
            _ => None,
        }
    }
}

impl FromSqlValue for DateTime<FixedOffset> {
    fn from_sql_value(sql_value: &SqlValue) -> Option<Self> {
        match *sql_value {
            SqlValue::TimestampTz(d, _) => Some(d),
            _ => None,
        }
    }
}

/// Creates a `DateTime<Utc>` from the Oracle format.
///
/// Oracle uses seven bytes for a date, and eleven bytes for a timestamp.
///
fn create_datetime_from_raw(data: &[u8]) -> DateTime<Utc> {
    let century = convert_century(data[0]);
    let year = convert_year(data[1]);
    let month = convert_month(data[2]);
    let day = convert_day(data[3]);
    let hour = convert_hour(data[4]);
    let minute = convert_minute(data[5]);
    let second = convert_second(data[6]);
    if data.len() <= 7 {
        Utc.ymd(century + year, month, day)
            .and_hms(hour, minute, second)
    } else {
        let nano = convert_nano(&data[7..11]);
        Utc.ymd(century + year, month, day)
            .and_hms_nano(hour, minute, second, nano)
    }
}

fn create_raw_from_date(date: Date<Utc>) -> [u8; 7] {
    let century = convert_year_to_century_raw(date.year());
    let year = convert_year_to_raw(date.year());
    let month = date.month() as u8;
    let day = date.day() as u8;
    let hour = convert_hour_to_raw(0);
    let minute = convert_minute_to_raw(0);
    let second = convert_second_to_raw(0);
    [century, year, month, day, hour, minute, second]
}

fn create_raw_from_datetime(datetime: &DateTime<Utc>) -> [u8; 11] {
    let century = convert_year_to_century_raw(datetime.year());
    let year = convert_year_to_raw(datetime.year());
    let month = datetime.month() as u8;
    let day = datetime.day() as u8;
    let hour = convert_hour_to_raw(datetime.hour());
    let minute = convert_minute_to_raw(datetime.minute());
    let second = convert_second_to_raw(datetime.second());
    let nano = convert_nano_to_raw(datetime.nanosecond());
    [
        century, year, month, day, hour, minute, second, nano[0], nano[1], nano[2], nano[3],
    ]
}

/// Creates a `DateTime<FixedOffset>` from the Oracle format.
///
/// Oracle uses thirteen bytes for a timestamp with timezone.
///
fn create_datetime_with_timezone_from_raw(data: &[u8]) -> DateTime<FixedOffset> {
    let century = convert_century(data[0]);
    let year = convert_year(data[1]);
    let month = convert_month(data[2]);
    let day = convert_day(data[3]);
    let hour = convert_hour(data[4]);
    let minute = convert_minute(data[5]);
    let second = convert_second(data[6]);
    let nano = convert_nano(&data[7..11]);
    let timezone_hour = convert_timezone_hour(data[11]);
    let timezone_minute = convert_timezone_minute(data[12]);
    let hour_in_secs = timezone_hour * 3600;
    let minutes_in_secs = timezone_minute * 60;
    let utc_dt = Utc
        .ymd(century + year, month, day)
        .and_hms_nano(hour, minute, second, nano);
    utc_dt.with_timezone(&FixedOffset::east(hour_in_secs + minutes_in_secs))
}

/// Creates an Oracle byte format from `DateTime<FixedOffset>`.
///
/// Oracle uses thirteen bytes for a timestamp with timezone.
/// Oracle holds the UTC time along with an offset. `DateTime<FixedOffset>` will report
/// back the date and hour as per the local time, so UTC values are needed instead.
///
fn create_raw_from_datetime_with_timezone(datetime: &DateTime<FixedOffset>) -> [u8; 13] {
    let utc = datetime.with_timezone(&Utc);
    let century = convert_year_to_century_raw(utc.year());
    let year = convert_year_to_raw(utc.year());
    let month = utc.month() as u8;
    let day = utc.day() as u8;
    let hour = convert_hour_to_raw(utc.hour());
    let minute = convert_minute_to_raw(utc.minute());
    let second = convert_second_to_raw(utc.second());
    let nano = convert_nano_to_raw(utc.nanosecond());
    let timezone_hour = convert_timezone_seconds_to_hour_raw(datetime.offset().local_minus_utc());
    let timezone_minutes =
        convert_timezone_seconds_to_minute_raw(datetime.offset().local_minus_utc());
    [
        century,
        year,
        month,
        day,
        hour,
        minute,
        second,
        nano[0],
        nano[1],
        nano[2],
        nano[3],
        timezone_hour,
        timezone_minutes,
    ]
}

fn convert_century(century_byte: u8) -> i32 {
    let number = i32::from(century_byte);
    (number - 100) * 100
}

fn convert_year_to_century_raw(year: i32) -> u8 {
    let byte = (year / 100) + 100;
    byte as u8
}

fn convert_year(year_byte: u8) -> i32 {
    let number = i32::from(year_byte);
    number - 100
}

fn convert_year_to_raw(year: i32) -> u8 {
    let decade = year - ((year / 100) * 100);
    let byte = decade + 100;
    byte as u8
}

fn convert_month(month_byte: u8) -> u32 {
    u32::from(month_byte)
}

fn convert_day(day_byte: u8) -> u32 {
    u32::from(day_byte)
}

fn convert_hour(hour_byte: u8) -> u32 {
    let number = u32::from(hour_byte);
    number - 1
}

fn convert_hour_to_raw(hour: u32) -> u8 {
    let byte = hour + 1;
    byte as u8
}

fn convert_minute(minute_byte: u8) -> u32 {
    let number = u32::from(minute_byte);
    number - 1
}

fn convert_minute_to_raw(minute: u32) -> u8 {
    let byte = minute + 1;
    byte as u8
}

fn convert_second(second_byte: u8) -> u32 {
    let number = u32::from(second_byte);
    number - 1
}

fn convert_second_to_raw(second: u32) -> u8 {
    let byte = second + 1;
    byte as u8
}

fn convert_nano(nano_bytes: &[u8]) -> u32 {
    BigEndian::read_u32(nano_bytes)
}

fn convert_nano_to_raw(nano: u32) -> [u8; 4] {
    let mut bytes = [0; 4];
    BigEndian::write_u32(&mut bytes, nano);
    bytes
}

/// Converts a byte into a timezone hour, as per the Oracle `Timestamp with time zone` format.
///
fn convert_timezone_hour(timezone_hour_byte: u8) -> i32 {
    let number = i32::from(timezone_hour_byte);
    number - 20
}

fn convert_timezone_seconds_to_hour_raw(timezone_seconds: i32) -> u8 {
    let hour = timezone_seconds / 3600;
    let byte = hour + 20;
    byte as u8
}

/// Converts a byte into timezone minutes, as per the Oracle `Timestamp with time zone` format.
///
fn convert_timezone_minute(timezone_minute_byte: u8) -> i32 {
    let number = i32::from(timezone_minute_byte);
    number - 60
}

fn convert_timezone_seconds_to_minute_raw(timezone_seconds: i32) -> u8 {
    let seconds = timezone_seconds - ((timezone_seconds / 3600) * 3600);
    let minutes = seconds / 60;
    let byte = minutes + 60;
    byte as u8
}
