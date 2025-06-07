package main

/*
#include "../../c/nylon.h"
*/
import "C"
import (
	"unsafe"

	"github.com/AssetsArt/easy-proxy/sdk/go/sdk"
)

func SendResponse(sdk_dispatcher *sdk.Dispatcher) C.FfiOutput {
	output := sdk_dispatcher.ToBytes()
	return C.FfiOutput{
		ptr: (*C.uchar)(C.CBytes(output)),
		len: C.ulong(len(output)),
	}
}

func InputToDispatcher(ptr *C.uchar, input_len C.int) *sdk.Dispatcher {
	input := C.GoBytes(unsafe.Pointer(ptr), C.int(input_len))
	dispatcher := sdk.WrapDispatcher(input)
	return dispatcher
}
