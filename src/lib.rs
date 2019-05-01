extern crate lazy_static;
extern crate pg_extend;
extern crate pg_extern_attr;

use std::error::Error;
use std::net::IpAddr;
use std::str::FromStr;
use std::sync::{Arc, Mutex};

use maxminddb::{geoip2, MaxMindDBError};
use maxminddb::MaxMindDBError::AddressNotFoundError;
use pg_extend::{pg_error, pg_magic};
use pg_extern_attr::pg_extern;

const DEFAULT_DB_PATH: &str = "/usr/share/GeoIP/GeoLite2-Country.mmdb";

/// Create an alias for GeoIP database type because this notation is UGLYYY
type GeoDB = maxminddb::Reader<Vec<u8>>;

/// Cache the database instance on first open
struct InstanceCache {
    db: Mutex<Option<Arc<GeoDB>>>
}

impl InstanceCache {
    fn new() -> InstanceCache {
        InstanceCache { db: Mutex::new(None) }
    }

    fn get(&self) -> Result<Arc<GeoDB>, Box<Error>> {
        let mut db = self.db.lock().unwrap();

        if let None = *db {
            *db = Some(Arc::new(GeoDB::open_readfile(DEFAULT_DB_PATH)?));
        }

        return Ok(db.as_ref().unwrap().clone());
    }
}

lazy_static::lazy_static! {
    static ref DB_MANAGER: InstanceCache = InstanceCache::new();
}

/// This tells Postgres this library is a Postgres extension
pg_magic!(version: pg_sys::PG_VERSION_NUM);

fn geoip_country_internal(value: &str) -> Result<Option<String>, Box<Error>> {
    let ip: IpAddr = FromStr::from_str(value)?;
    let geoip_db = DB_MANAGER.get()?;

    let result: Result<geoip2::Country, MaxMindDBError> = geoip_db.lookup(ip);
    match result {
        Ok(ret) => Ok(ret.country.unwrap().iso_code),
        Err(AddressNotFoundError(_e)) => Ok(None),
        Err(e) => Err(e.into())
    }
}

#[pg_extern]
fn geoip_country(value: String) -> String {
    match geoip_country_internal(&value)
    {
        Ok(Some(result)) => result,
        Ok(None) => "N/A".to_string(), // FIXME return SQL NULL here
        Err(e) => {
            pg_error::log(
                pg_error::Level::Error,
                file!(),
                line!(),
                module_path!(),
                e.description()
            );
            return "N/A".to_string()
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_country() {
        assert_eq!(geoip_country_internal("8.8.8.8").unwrap(),
                   Some("US".to_string()));
        assert_eq!(geoip_country_internal("127.0.0.1").unwrap(),
                   None);
    }

    #[test]
    fn test_cache() {
        // DB instance is not initialized at first.
        let c = InstanceCache::new();
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
