package main

import "C"

import (
	"context"
	"embed"
	"fmt"
	"io/ioutil"
	"os"
	"strings"
	"unicode"

	"github.com/tetratelabs/wazero"
	"github.com/tetratelabs/wazero/imports/wasi_snapshot_preview1"
)

var rootFS embed.FS

// main writes an input file to stdout, just like `cat`.
//
// This is a basic introduction to the WebAssembly System Interface (WASI).
// See https://github.com/WebAssembly/WASI
func main() {
	runWASIFile(os.Args[1])
}

//export runWASIFile
func runWASIFile(filename string) {
	// Remove unprinable chars from C FFI call
	filename = strings.Map(func(r rune) rune {
		if unicode.IsPrint(r) {
			return r
		}
		return -1
	}, filename)

	b, err := ioutil.ReadFile(filename)
	if err != nil {
		fmt.Print(err)
	}

	runWASI(b)
}

func runWASI(wasmBytes []byte) {
	// Choose the context to use for function calls.
	ctx := context.Background()

	// Create a new WebAssembly Runtime.
	r := wazero.NewRuntimeWithConfig(ctx, wazero.NewRuntimeConfigInterpreter())
	defer r.Close(ctx) // This closes everything this Runtime created.

	// Combine the above into our baseline config, overriding defaults.
	config := wazero.NewModuleConfig().
		// By default, I/O streams are discarded and there's no file system.
		WithStdout(os.Stdout).WithStderr(os.Stderr) //		.WithStdin(os.Stdin).WithFS(rootFS)

	// Instantiate WASI, which implements system I/O such as console output.
	wasi_snapshot_preview1.MustInstantiate(ctx, r)

	// Choose the binary we want to test. Most compilers that implement WASI
	// are portable enough to use binaries interchangeably.

	// InstantiateModule runs the "_start" function, WASI's "main".
	// * Set the program name (arg[0]) to "wasi"; arg[1] should be "/test.txt".

	_, err := r.InstantiateWithConfig(ctx, wasmBytes, config)
	if err != nil {
		fmt.Print(err)
	}
}
