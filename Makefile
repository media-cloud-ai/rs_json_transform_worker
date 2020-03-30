.PHONY: docker-build docker-clean docker-registry-login docker-push-registry test check-version clippy

DOCKER_REGISTRY?=
DOCKER_IMG_NAME?=media-cloud-ai/rs_json_transform_worker
ifneq ($(DOCKER_REGISTRY), ) 
	DOCKER_IMG_NAME := /${DOCKER_IMG_NAME}
endif

DATA_FOLDER ?= ${PWD}/../data/
VERSION?=`cat Cargo.toml  | grep -oP '^version[^"]*"\K[^"]*'`

docker = docker run -v ${PWD}:/sources -v ${DATA_FOLDER}:/sources/data --rm ${DOCKER_REGISTRY}${DOCKER_IMG_NAME}:${VERSION}

docker-build:
	@docker build -t ${DOCKER_REGISTRY}${DOCKER_IMG_NAME}:${VERSION} -f Dockerfile .

docker-clean:
	@docker rmi ${DOCKER_REGISTRY}${DOCKER_IMG_NAME}:${VERSION}

docker-registry-login:
	@docker login --username "${CI_REGISTRY_USER}" -p ${CI_REGISTRY_PASSWORD} ${DOCKER_REGISTRY}

docker-push-registry:
	@docker push ${DOCKER_REGISTRY}${DOCKER_IMG_NAME}:${VERSION}

bash:
	@docker run -it ${DOCKER_REGISTRY}${DOCKER_IMG_NAME}:${VERSION} bash

check-version:
	@$(eval code := $(shell export DOCKER_CLI_EXPERIMENTAL=enabled; \
		docker manifest inspect ${DOCKER_REGISTRY}${DOCKER_IMG_NAME}:${VERSION} > /dev/null \
		&& echo 0 || echo 1))
	@if [ "${code}" = "0" ]; then \
		echo "image ${DOCKER_REGISTRY}${DOCKER_IMG_NAME}:${VERSION} already exists."; exit 1;\
	else \
		echo "image ${DOCKER_REGISTRY}${DOCKER_IMG_NAME}:${VERSION} is available."; \
	fi
