/*
 * vivy_abi.h — DIVA embedded ABI v1 contract constants.
 *
 * The function prototypes come from the cgo-generated header emitted beside
 * the shared library (go build -buildmode=c-shared -o <name>.so produces
 * <name>.h). This hand-maintained companion header is the single ABI version
 * source consumed by the Rust bridge; cmd/vivy-shared asserts it stays in
 * sync with abiVersion in exports.go.
 */
#ifndef VIVY_ABI_H
#define VIVY_ABI_H

#define VIVY_ABI_VERSION 1

#endif
