#[macro_use] extern crate lazy_static;
extern crate pg_extend;
extern crate pg_extern_attr;

use std::ffi::CString;
use std::net::IpAddr;
use std::str::FromStr;

use maxminddb::{geoip2, MaxMindDBError};
use maxminddb::MaxMindDBError::AddressNotFoundError;
use pg_extend::{pg_magic, pg_error};
use pg_extern_attr::pg_extern;
use std::error::Error;

lazy_static! {
    static ref geoip_db: Result<maxminddb::Reader<Vec<u8>>, MaxMindDBError>
        = maxminddb::Reader::open_readfile("/usr/share/GeoIP/GeoLite2-Country.mmdb");
}

/// This tells Postgres this library is a Postgres extension
pg_magic!(version: pg_sys::PG_VERSION_NUM);

fn geoip_country_internal(value: CString) -> Result<Option<CString>, Box<Error>> {
    let ip: IpAddr = FromStr::from_str(value.to_str()?)?;

    let result: Result<geoip2::Country, MaxMindDBError> = geoip_db?.lookup(ip);
    match result {
        Ok(ret) => Ok(Some(CString::new(ret.country.unwrap().iso_code.unwrap())?)),
        Err(AddressNotFoundError(_e)) => Ok(None),
        Err(e) => Err(e.into())
    }
}

#[pg_extern]
fn geoip_country(value: CString) -> CString {
    match geoip_country_internal(value)
    {
        Ok(Some(result)) => result,
        Ok(None) => CString::new("N/A").unwrap(), // FIXME return SQL NULL here
        Err(e) => {
            pg_error::log(
                pg_error::Level::Error,
                file!(),
                line!(),
                module_path!(),
                e.description()
            );
            return CString::new("N/A").unwrap()
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_country() {
        assert_eq!(geoip_country_internal(CString::new("8.8.8.8").unwrap()).unwrap(),
                   Some(CString::new("US").unwrap()));
        assert_eq!(geoip_country_internal(CString::new("127.0.0.1").unwrap()).unwrap(),
                   None);
    }
}
