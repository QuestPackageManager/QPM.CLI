$ENV:TRYCMD="overwrite"
& cargo test --bin qpm -- tests::commands::trycmd -- --nocapture
$ENV:TRYCMD=""
