package sdk

/*
#include <stdlib.h>
#include <string.h>
*/
import "C"
import (
	"sync"
	"unsafe"
)

// BufferPool manages reusable C buffers to reduce malloc/free overhead
type BufferPool struct {
	pools map[int]*sync.Pool
	mu    sync.RWMutex
}

var globalBufferPool = &BufferPool{
	pools: make(map[int]*sync.Pool),
}

// Size buckets for pooling (powers of 2)
var sizeBuckets = []int{
	64, 128, 256, 512, 1024, 2048, 4096, 8192, 16384, 32768,
}

func (bp *BufferPool) getPoolForSize(size int) *sync.Pool {
	// Find appropriate bucket
	bucketSize := 64
	for _, bs := range sizeBuckets {
		if size <= bs {
			bucketSize = bs
			break
		}
	}
	if size > sizeBuckets[len(sizeBuckets)-1] {
		bucketSize = size // Use exact size for large allocations
	}

	bp.mu.RLock()
	pool, exists := bp.pools[bucketSize]
	bp.mu.RUnlock()

	if !exists {
		bp.mu.Lock()
		// Double-check after acquiring write lock
		pool, exists = bp.pools[bucketSize]
		if !exists {
			finalSize := bucketSize
			pool = &sync.Pool{
				New: func() interface{} {
					return C.malloc(C.size_t(finalSize))
				},
			}
			bp.pools[bucketSize] = pool
		}
		bp.mu.Unlock()
	}

	return pool
}

// Get retrieves a buffer from the pool or allocates a new one
func (bp *BufferPool) Get(size int) (*C.uchar, int) {
	if size == 0 {
		return nil, 0
	}

	pool := bp.getPoolForSize(size)
	ptr := pool.Get().(unsafe.Pointer)
	return (*C.uchar)(ptr), size
}

// Put returns a buffer to the pool
func (bp *BufferPool) Put(ptr *C.uchar, size int) {
	if ptr == nil || size == 0 {
		return
	}

	pool := bp.getPoolForSize(size)
	pool.Put(unsafe.Pointer(ptr))
}

// GetBuffer gets a pooled buffer for FFI data transfer
func GetBuffer(data []byte) (*C.uchar, int) {
	dataLen := len(data)
	if dataLen == 0 {
		return nil, 0
	}

	dataPtr, actualSize := globalBufferPool.Get(dataLen)
	if dataPtr != nil {
		C.memcpy(unsafe.Pointer(dataPtr), unsafe.Pointer(&data[0]), C.size_t(dataLen))
	}
	return dataPtr, actualSize
}

// PutBuffer returns a buffer to the pool
func PutBuffer(ptr *C.uchar, size int) {
	globalBufferPool.Put(ptr, size)
}
