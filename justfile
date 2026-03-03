js_dirs := "packages/eth-rpc packages/railgun-js packages/tc-js"

wasm:
    for dir in {{js_dirs}}; do (cd $dir && just wasm); done
