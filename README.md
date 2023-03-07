# ever-tl

Serialization/deserialization of TL-based data types

## Table of Contents

- [About](#about)
- [Getting Started](#getting-started)
- [Usage](#usage)
- [Contributing](#contributing)
- [License](#license)

## About

Implementation of Reliable Large Datagram Protocol (RLDP) in safe Rust. RLDP is a protocol that runs on top of ADNL UDP, which is used to transfer large data blocks and includes Forward Error Correction (FEC) algorithms as a replacement of acknowledgment packets on the other side. This makes it possible to transfer data between network components more efficiently, but with more traffic consumption.

## Getting Started

### Prerequisites

Rust complier v1.65+.

### Installing

```
git clone --recurse-submodules https://github.com/tonlabs/ever-rldp.git
cd ever-rldp
cargo build --release
```

## Usage

This project output is the library which is used as a part of Everscale/Venom node. Also it can be used in standalone tools.

## Contributing

Contribution to the project is expected to be done via pull requests submission.

## License

See the [LICENSE](LICENSE) file for details.
