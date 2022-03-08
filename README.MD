# program used for proxy GPU work


# Build

``` shell
cargo build --release
```


# Quick start

## run gpuproxy

```shell
gpuproxy --url <ip>:<port> --log-level <lv> run --db-dsn <database url> --disable-worker <true/false> --fs-resource-path <path> --allow-types <task type> --max-tasks <int> --no-gpu <true/false> --resource-type <db/fs>
```
1. url: the endpoint that the gpuproxy listens
2. log-level: program log level
3. db-dsn: database connecton string (support sqlite3/mysql)
4. disable-worker: whether to prohibit the gruproxy from running as a worker
5. fs-resource-path: if the resources are stored in file system, then configure the folder path here
6. max-tasks:  how many tasks can run at most
7. no-gpu: whether to use gpu
8. resource-type: resource type (fs/db)
9. allow-types: task types supported by workers

### run gpuproxy_worker

```shell
gpuproxy_worker run --url http://<ip>:<port> --db-dsn <database url> --fs-resource-path <path> --allow-types <task type> --max-tasks <int> --no-gpu <true/false> --resource-type <db/fs>
```

1. url:  the endpoint that the gpuproxy, connect to the gpuproxy to get the task and submit the execution result
2. log-level: program log level
3. db-dsn: database connecton string sqlite3
4. fs-resource-path: if the resources are stored in file system, then configure the folder path here
5. max-tasks:  how many tasks can run at most
6. no-gpu: whether to use gpu
7. resource-type: resource type (fs/db)
8。 allow-types: task types supported by workers

### run the gpu proxy plugin with venus-cluster

the plugin needs to be configured into the venus-worker configuration file, loaded and run as a plugin

```toml
[[processors.c2]]
bin="<cluster_c2_plugin 插件路径>"
args = ["run", "--gpuproxy-url", "http://127.0.0.1:18888","--log-level", "trace"]  #插件参数
```

after the program runs, the c2 task will be automatically submitted to the task center, and the final result will be continuously polled