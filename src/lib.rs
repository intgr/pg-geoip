extern crate lazy_static;
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
use std::sync::{Arc, Mutex};

const DEFAULT_DB_PATH: &str = "/usr/share/GeoIP/GeoLite2-Country.mmdb";

/// Cache the database instance on first open
struct DatabaseCache {
    db: Mutex<Option<Arc<maxminddb::Reader<Vec<u8>>>>>
}

impl DatabaseCache {
    fn new() -> DatabaseCache {
        DatabaseCache { db: Mutex::new(None) }
    }

    fn get(&self) -> Result<Arc<maxminddb::Reader<Vec<u8>>>, Box<Error>> {
        let mut db = self.db.lock().unwrap();

        if let None = *db {
            *db = Some(Arc::new(maxminddb::Reader::open_readfile(DEFAULT_DB_PATH)?));
        }

        return Ok(db.as_ref().unwrap().clone());
    }
}

lazy_static::lazy_static! {
    static ref DB_MANAGER: DatabaseCache = DatabaseCache::new();
}

/// This tells Postgres this library is a Postgres extension
pg_magic!(version: pg_sys::PG_VERSION_NUM);

fn geoip_country_internal(value: CString) -> Result<Option<CString>, Box<Error>> {
    let ip: IpAddr = FromStr::from_str(value.to_str()?)?;
    let geoip_db = DB_MANAGER.get()?;

    let result: Result<geoip2::Country, MaxMindDBError> = geoip_db.lookup(ip);
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

    #[test]
    fn test_cache() {
        // DB instance is not initialized at first.
        let c = DatabaseCache::new();
        assert!(c.db.lock().unwrap().is_none());

        // Get first intsance, which initializes it.
        let db = c.get().unwrap();
        assert!(c.db.lock().unwrap().is_some());
        assert_eq!(Arc::strong_count(&db), 2);

        // Make sure references are shared
        let db2 = c.get().unwrap();
        assert_eq!(Arc::strong_count(&db2), 3);
    }
}
