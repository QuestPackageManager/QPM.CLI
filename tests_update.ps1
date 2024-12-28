$ENV:TRYCMD="overwrite"
& cargo build --bin qpm 
& cargo test --bin qpm -- tests::commands::trycmd -- --nocapture
$ENV:TRYCMD=""
