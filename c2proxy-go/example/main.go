package main

import (
	"context"
	c2proxy_go "github.com/hunjixin/c2proxy/c2proxy-go"
	"log"
	"time"
)

func main() {
	ctx := context.TODO()
	client, closer, err := c2proxy_go.NewC2ProxyClient(ctx, "http://127.0.0.1:8888")
	if err != nil {
		log.Fatal(err)
		return
	}
	defer closer()

	taskId, err := client.SubmitTask([]byte{}, "f01001", [32]byte{}, 10)
	if err != nil {
		log.Fatal(err)
		return
	}

	for {
		task, err := client.GetTask(taskId)
		if err != nil {
			log.Fatal(err)
			return
		}
		if task.Status == c2proxy_go.Completed {

		}
		time.Sleep(time.Second * 5)
	}
}
