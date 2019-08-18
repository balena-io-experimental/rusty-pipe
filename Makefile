.PHONY: docker

docker:
	docker build -t rusty-pipe:dev -f docker/Dockerfile .
