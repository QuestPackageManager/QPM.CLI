$ENV:TRYCMD="overwrite"
& cargo test --test tests::commands::trycmd
$ENV:TRYCMD=""
