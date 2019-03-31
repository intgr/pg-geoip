extern crate pg_extern_attr;
extern crate pg_extend;

use pg_extern_attr::pg_extern;
use pg_extend::pg_magic;
use std::ffi::{CString};
use geoip::{GeoIp, DBType, Options, IpAddr};

/// This tells Postgres this library is a Postgres extension
pg_magic!(version: pg_sys::PG_VERSION_NUM);

fn geoip_country_internal(value: CString) -> Option<CString> {
    let geoip = GeoIp::open_type(DBType::CountryEdition, Options::Standard).unwrap();

    // geoip.city_info_by_ip();
    let ip = IpAddr::V4(value.to_str().unwrap().parse().unwrap());
    println!("IP: {:?}", ip);
    // let res = geoip.city_info_by_ip(ip).unwrap();
    // return Some(CString::new(res.country_code.unwrap()).unwrap());
    match geoip.country_code_by_ip(ip)
    {
        Some(cc) => Some(CString::new(cc).unwrap()),
        None => None
    }
    // return Some(CString::new(res.unwrap()).unwrap());
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
                   CString::new("Error").unwrap());
    }
}
