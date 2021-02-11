#!/usr/bin/bash

if [[ ! -d ./telegram-bot/ ]]; then
  git clone https://github.com/telegram-rs/telegram-bot && cd telegram-bot && git pull origin pull/228/head
  cd ..
fi

cargo $@
