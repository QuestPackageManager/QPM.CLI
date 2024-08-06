$ENV:TRYCMD="dump"
& cargo test --bin qpm -- tests::commands::trycmd -- --nocapture
$ENV:TRYCMD=""