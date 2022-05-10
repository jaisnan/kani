// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT

#[kani::proof]
#[kani::unwind(9)]
fn main() {
    let i: i32 = kani::any();
    kani::assume(i < 10);
    kani::expect_fail(i < 20, "Blocked by assumption above.");
    let mut counter = 0;
    loop {
        counter += 1;
        assert!(counter < 10);
    }
}
