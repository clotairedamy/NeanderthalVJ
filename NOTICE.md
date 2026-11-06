# Notices

NeanderthalVJ is licensed under either of the [Apache License, Version
2.0](LICENSE-APACHE) or the [MIT license](LICENSE-MIT), at your option.
Unless you explicitly state otherwise, any contribution you submit for
inclusion in the work, as defined in the Apache-2.0 license, shall be dual
licensed as above, without any additional terms or conditions.

It includes or uses the following work by others, under their own terms.

## The head scan

`assets/LeePerrySmith.glb` and `assets/Infinite-Level_02_Tangent_SmoothUV.jpg`:
*Infinite, 3D Head Scan* by Lee Perry-Smith, licensed under the [Creative
Commons Attribution 3.0 Unported License](https://creativecommons.org/licenses/by/3.0/),
based on a work at www.triplegangers.com. The licence text that came with
the scan is in `assets/LeePerrySmith_License.txt`.

## ADS

`crates/ads` is the Rust edition of ADS, the Artelier design system, by
Clotaire Damy, contributed to this project under its licence.

## Syphon

The optional Syphon output (the `syphon` build feature, macOS only) builds
the [Syphon Framework](https://github.com/Syphon/Syphon-Framework) from
source, through the [`syphon-rs`](https://crates.io/crates/syphon-rs)
bindings, and links it into the app. Its licence:

```
Syphon Framework License:

Copyright 2010 bangnoise (Tom Butterworth) & vade (Anton Marini).
All rights reserved.

Redistribution and use in source and binary forms, with or without
modification, are permitted provided that the following conditions are met:

* Redistributions of source code must retain the above copyright
notice, this list of conditions and the following disclaimer.

* Redistributions in binary form must reproduce the above copyright
notice, this list of conditions and the following disclaimer in the
documentation and/or other materials provided with the distribution.

* Neither the name of the Syphon Project nor the names of its contributors
may be used to endorse or promote products derived from this software
without specific prior written permission.

THIS SOFTWARE IS PROVIDED BY THE COPYRIGHT HOLDERS AND CONTRIBUTORS "AS IS" AND
ANY EXPRESS OR IMPLIED WARRANTIES, INCLUDING, BUT NOT LIMITED TO, THE IMPLIED
WARRANTIES OF MERCHANTABILITY AND FITNESS FOR A PARTICULAR PURPOSE ARE
DISCLAIMED. IN NO EVENT SHALL THE COPYRIGHT HOLDERS BE LIABLE FOR ANY
DIRECT, INDIRECT, INCIDENTAL, SPECIAL, EXEMPLARY, OR CONSEQUENTIAL DAMAGES
(INCLUDING, BUT NOT LIMITED TO, PROCUREMENT OF SUBSTITUTE GOODS OR SERVICES;
LOSS OF USE, DATA, OR PROFITS; OR BUSINESS INTERRUPTION) HOWEVER CAUSED AND
ON ANY THEORY OF LIABILITY, WHETHER IN CONTRACT, STRICT LIABILITY, OR TORT
(INCLUDING NEGLIGENCE OR OTHERWISE) ARISING IN ANY WAY OUT OF THE USE OF THIS
SOFTWARE, EVEN IF ADVISED OF THE POSSIBILITY OF SUCH DAMAGE.
```

## Fonts

The interface loads Bricolage Grotesque, Inter Tight, Martian Mono and
JetBrains Mono from Google Fonts at run time. They are licensed under the
SIL Open Font License 1.1 and are not distributed with this project.

## Rust crates

Every crate this project depends on is listed in `Cargo.lock`, and each
carries its own licence. `cargo about` or `cargo deny` will produce the
full list.
