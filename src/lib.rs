extern crate pg_extend;
extern crate pg_extern_attr;

use std::ffi::CString;
use std::net::IpAddr;
use std::str::FromStr;

use maxminddb::geoip2;
use maxminddb::MaxMindDBError::AddressNotFoundError;
use pg_extend::pg_magic;
use pg_extern_attr::pg_extern;

/// This tells Postgres this library is a Postgres extension
pg_magic!(version: pg_sys::PG_VERSION_NUM);

fn geoip_country_internal(value: CString) -> Option<CString> {
    let geoip = maxminddb::Reader::open_readfile("/usr/share/GeoIP/GeoLite2-Country.mmdb").unwrap();

    let ip: IpAddr = FromStr::from_str(value.to_str().unwrap()).unwrap();
    println!("IP: {:?}", ip);
    let ret: geoip2::Country = match geoip.lookup(ip) {
        Ok(ret) => ret,
        Err(AddressNotFoundError(_e)) => return None,
        Err(e) => panic!(e)
    };

    Some(CString::new(ret.country?.iso_code?).unwrap())
}

#[pg_extern]
fn geoip_country(value: CString) -> CString {
    match geoip_country_internal(value)
    {
        Some(cc) => cc,
        None => CString::new("Error").unwrap()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_country() {
        assert_eq!(geoip_country(CString::new("1.2.3.4").unwrap()),
                   CString::new("US").unwrap());
        assert_eq!(geoip_country(CString::new("127.0.0.1").unwrap()),
                   CString::new("Error").unwrap()); // FIXME hack.
    }
}
