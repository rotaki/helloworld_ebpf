set -euxo

cargo run --release --config 'target."cfg(all())".runner="sudo -E"' -- --function sequential-direct                                              > result-sequential-direct.txt
cargo run --release --config 'target."cfg(all())".runner="sudo -E"' -- --function sequential-direct-with-fallocate  --fallocate-flag none        > result-sequential-direct-with-fallocate-none.txt
cargo run --release --config 'target."cfg(all())".runner="sudo -E"' -- --function sequential-direct-with-fallocate  --fallocate-flag keep-size   > result-sequential-direct-with-fallocate-keep-size.txt
cargo run --release --config 'target."cfg(all())".runner="sudo -E"' -- --function sequential-direct-with-fallocate  --fallocate-flag zero-range  > result-sequential-direct-with-fallocate-zero-range.txt
cargo run --release --config 'target."cfg(all())".runner="sudo -E"' -- --function sequential-direct-double-writes                                > result-sequential-direct-double-writes.txt
cargo run --release --config 'target."cfg(all())".runner="sudo -E"' -- --function random-direct                                                  > result-random-direct.txt
cargo run --release --config 'target."cfg(all())".runner="sudo -E"' -- --function random-direct-with-fallocate      --fallocate-flag none        > result-random-direct-with-fallocate-none.txt
cargo run --release --config 'target."cfg(all())".runner="sudo -E"' -- --function random-direct-with-fallocate      --fallocate-flag keep-size   > result-random-direct-with-fallocate-keep-size.txt
cargo run --release --config 'target."cfg(all())".runner="sudo -E"' -- --function random-direct-with-fallocate      --fallocate-flag zero-range  > result-random-direct-with-fallocate-zero-range.txt
cargo run --release --config 'target."cfg(all())".runner="sudo -E"' -- --function random-direct-double-writes                                    > result-random-direct-double-writes.txt