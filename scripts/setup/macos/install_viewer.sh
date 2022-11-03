#!/bin/bash
# Copyright Kani Contributors
# SPDX-License-Identifier: Apache-2.0 OR MIT

set -eu

# Install cbmc-viewer

# Source kani-dependencies to get CBMC_VIEWER_VERSION
source kani-dependencies

if [ -z "${CBMC_VIEWER_VERSION:-}" ]; then
  echo "$0: Error: CBMC_VIEWER_VERSION is not specified"
  exit 1
fi

set -x

brew tap aws/tap
brew install cbmc-viewer@${CBMC_VERSION}
