//! Every SQL statement core runs, as constants generated from
//! `queries/<kind>/*.sql` by `build.rs`, which checks each one against the
//! schema its migrations build.

include!(concat!(env!("OUT_DIR"), "/sql.rs"));
