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

test-package:
	@${docker} py.test

bash:
	@docker run -it ${DOCKER_REGISTRY}${DOCKER_IMG_NAME}:${VERSION} bash
