docker-build:
	docker build . --tag dump-map-build
	docker run --rm -it --mount type=bind,source="$(CURDIR)",target=/app dump-map-build