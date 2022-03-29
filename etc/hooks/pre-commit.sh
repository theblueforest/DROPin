#!/bin/bash

set -e

ROOT_DIR=$(realpath $(dirname ${BASH_SOURCE[0]}))/../..

SRC_DIR=$ROOT_DIR/src

COPYING_HEADER="\/*     _              _ _\n *  __| |_ _ ___ _ __( |_)_ _\n * \/ _\` | '_\/ _ \\\\ '_ \\\\\/| | ' \\\\\n * \\\\__,_|_| \\\\___\/ .__\/ |_|_||_| dropin-compiler - WebAssembly\n *              |_|\n * Copyright \© 2019-2022 Blue Forest\n * \n * This program is free software: you can redistribute it and\/or modify\n * it under the terms of the GNU Affero General Public License as published\n * by the Free Software Foundation under version 3 of the License.\n * \n * This program is distributed in the hope that it will be useful,\n * but WITHOUT ANY WARRANTY; without even the implied warranty of\n * MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE. See the\n * GNU Affero General Public License for more details.\n * \n * You should have received a copy of the GNU Affero General Public License\n * along with this program. If not, see <https:\/\/www.gnu.org\/licenses\/>.\n *\/\n\n"

for i in $(\
  git status --porcelain=1 \
  | awk '{ if (($1 == "A") && ($2 ~ /\.(rs|pest)$/)) { print $2 } }' \
); do
  if ! grep -q "Copyright © 2019-2022 Blue Forest" $i; then
    echo "Adding license header to $i"
    sed -i "1s/^/$COPYING_HEADER/" $i
    git add $i
  fi
done
