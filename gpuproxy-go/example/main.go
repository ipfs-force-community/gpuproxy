package main

import (
	"context"
	"encoding/json"
	"io/ioutil"
	"log"
	"time"

	"github.com/filecoin-project/go-address"
	c2proxy_go "github.com/hunjixin/gpuproxy/gpuproxy-go"
)

type Commit2In struct {
	SectorNum  int64
	Phase1Out  []byte
	SectorSize uint64
	Miner      address.Address
}

func main() {
	ctx := context.TODO()
	client, closer, err := c2proxy_go.NewC2ProxyClient(ctx, "http://127.0.0.1:8888")
	if err != nil {
		log.Fatal(err)
		return
	}
	defer closer()

	var commit2In Commit2In
	eightMiB, err := ioutil.ReadFile("./2KiB.json")
	if err != nil {
		log.Fatal(err)
		return
	}
	err = json.Unmarshal(eightMiB, &commit2In)
	if err != nil {
		log.Fatal(err)
		return
	}

	var proverId [32]byte
	copy(proverId[:], commit2In.Miner.Payload())
	taskId, err := client.SubmitC2Task(commit2In.Phase1Out, commit2In.Miner.String(), proverId, commit2In.SectorNum)
	if err != nil {
		log.Fatal(err)
		return
	}
	return

	for {
		task, err := client.GetTask(taskId)
		if err != nil {
			log.Fatal(err)
			return
		}
		if task.State == c2proxy_go.Completed {
			log.Println("task ", task.Id, " has been complete by ", task.WorkerId)
			break
		}
		time.Sleep(time.Second * 5)
	}
}
