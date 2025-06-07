//go:build cgo

package main

/*
#include "../../c/nylon.h"
*/
import "C"
import (
	"unsafe"
)

//export plugin_free
func plugin_free(ptr *C.uchar) {
	C.free(unsafe.Pointer(ptr))
}

func main() {}
