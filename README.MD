# program used for proxy GPU work


# Build

``` shell
cargo build --release
```


# Quick start

```shell
./gpuroxy run
```

# Start worker

if have many machine,  build gpuproxy-worker and start to work

```shell
./gpuproxy-worker run
```

# send mock task

```shell
go run ./gpuproxy-go/example/main.go
```