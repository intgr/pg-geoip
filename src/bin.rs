// #[macro_use] extern crate lazy_static;

extern crate pg_extend;

use pg_extend::pg_create_stmt_bin;

pg_create_stmt_bin!(
    geoip_country_pg_create_stmt
);
