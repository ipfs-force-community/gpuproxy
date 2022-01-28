package gpuproxy_go

import (
	"context"
	"net/http"

	"github.com/filecoin-project/go-jsonrpc"
)

type TaskStatus int32

const (
	Undefined TaskStatus = iota
	Init      TaskStatus = 1
	Running   TaskStatus = 2
	Error     TaskStatus = 3
	Completed TaskStatus = 4
)

type Task struct {
	Id           string
	Miner        string
	ProveId      string
	SectorId     int64
	Phase1Output string
	Proof        []byte
	WorkerId     string
	TaskType     int32
	ErrorMsg     string
	Status       TaskStatus
	CreateAt     int64
	StartAt      int64
	CompleteAt   int64
}
type C2ProxyWorker interface {
	FetchTodo(workerId string) (Task, error)
	RecordProof(workerId string, tid string, proof string) (bool, error)
	RecordError(workerId string, tid string, errMsg string) (bool, error)
}

type C2ProxyClient interface {
	SubmitTask(phase1_output []byte, miner string, prover_id [32]byte, sector_id int64) (string, error)
	GetTask(id string) (Task, error)
}

type C2Proxy interface {
	C2ProxyWorker
	C2ProxyClient
}

func NewC2ProxyClient(ctx context.Context, url string) (C2Proxy, jsonrpc.ClientCloser, error) {
	impl := &C2ProxyStruct{}
	closer, err := jsonrpc.NewMergeClient(ctx, url, "Proof", []interface{}{
		&impl.C2ProxyWorkerStruct.Internal,
		&impl.C2ProxyClientStruct.Internal,
	}, http.Header{})
	if err != nil {
		return nil, nil, err
	}
	return impl, closer, nil
}
