FROM ghcr.io/cirruslabs/flutter:stable
WORKDIR /app
RUN flutter create --platforms=web --project-name horizon_comparison .
COPY lib/main.dart lib/main.dart
COPY test/comparison_test.dart test/comparison_test.dart
EXPOSE 8080
CMD ["flutter", "run", "-d", "web-server", "--web-hostname", "0.0.0.0", "--web-port", "8080"]
