#! /usr/bin/env sh

nvim --api-info | python -c 'import msgpack, sys, yaml; yaml.dump(msgpack.unpackb(sys.stdin.buffer.read()), sys.stdout)' > api_info.yml
