.PHONY: style

style:
	@python3.6 -m flake8 --ignore=W503,W606 --max-line-length=120 .

