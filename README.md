To build, you require [protoc](https://github.com/google/protobuf/releases/) to be installed on your system.

To run, you require a default installation of [FoundationDB](https://www.foundationdb.org/download/) to be installed and running on localhost.

This repo contains an unnecessarily networked service that reads your inputs interactively, sends them over the wire securely to itself, via gRPC, using TLS supported by a self-signed cert, and saves them in FoundationDB. Then, next time you enter something, it reads back what you had written previously, in gRPC format.
