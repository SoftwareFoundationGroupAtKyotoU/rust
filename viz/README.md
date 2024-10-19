# Visualizer

## Running

```shell
# from root of repository
mkdir -p .local/dumps
rm -f .local/dumps/*
./x.py run miri --stage 1 --args .local/loop.rs > .local/log.txt 2>&1

# from ./frontend
npm ci
npm run dev

# from ./backend
npm ci
ROOT_DIRECTORY=../../.local/dumps npm start
```
