package sdk

import (
	"fmt"
	"runtime"
	"sync"
)

// WorkerPool manages a pool of workers to reduce goroutine spawning overhead
type WorkerPool struct {
	tasks    chan func()
	wg       sync.WaitGroup
	size     int
	shutdown chan struct{}
	once     sync.Once
}

var defaultWorkerPool *WorkerPool

func init() {
	// Create default worker pool with CPU-count workers
	numWorkers := runtime.NumCPU() * 2
	if numWorkers < 4 {
		numWorkers = 4
	}
	defaultWorkerPool = NewWorkerPool(numWorkers)
}

// NewWorkerPool creates a new worker pool with the specified number of workers
func NewWorkerPool(size int) *WorkerPool {
	if size < 1 {
		size = 1
	}

	pool := &WorkerPool{
		tasks:    make(chan func(), size*4), // Buffered channel
		size:     size,
		shutdown: make(chan struct{}),
	}

	// Start workers
	for i := 0; i < size; i++ {
		pool.wg.Add(1)
		go pool.worker()
	}

	return pool
}

// worker is the main worker goroutine
func (p *WorkerPool) worker() {
	defer p.wg.Done()

	for {
		select {
		case task := <-p.tasks:
			if task != nil {
				task()
			}
		case <-p.shutdown:
			return
		}
	}
}

// Submit submits a task to the worker pool
func (p *WorkerPool) Submit(task func()) error {
	select {
	case p.tasks <- task:
		return nil
	case <-p.shutdown:
		return ErrPoolShutdown
	default:
		// If pool is full, execute in new goroutine (fallback)
		go task()
		return nil
	}
}

// SubmitBlocking submits a task and blocks if the pool is full
func (p *WorkerPool) SubmitBlocking(task func()) error {
	select {
	case p.tasks <- task:
		return nil
	case <-p.shutdown:
		return ErrPoolShutdown
	}
}

// Shutdown gracefully shuts down the worker pool
func (p *WorkerPool) Shutdown() {
	p.once.Do(func() {
		close(p.shutdown)
		p.wg.Wait()
	})
}

// GetDefaultWorkerPool returns the default global worker pool
func GetDefaultWorkerPool() *WorkerPool {
	return defaultWorkerPool
}

var ErrPoolShutdown = fmt.Errorf("worker pool is shutdown")
