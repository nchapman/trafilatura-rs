# frozen_string_literal: true

require_relative "lib/trafilatura/version"

Gem::Specification.new do |spec|
  spec.name    = "trafilatura"
  spec.version = Trafilatura::VERSION
  spec.authors = ["Nathaniel Chapman"]
  spec.license = "Apache-2.0"

  spec.summary     = "Extract readable content, comments, and metadata from web pages"
  spec.description = "High-performance web content extraction powered by Rust with native Ruby bindings via FFI."
  spec.homepage    = "https://github.com/nchapman/trafilatura-rs"

  spec.required_ruby_version = ">= 3.0"
  spec.add_dependency "ffi", "~> 1.15"

  if ENV["GEM_PLATFORM"]
    spec.platform = Gem::Platform.new(ENV["GEM_PLATFORM"])
  end

  spec.files         = Dir["lib/**/*", "README.md", "LICENSE"]
  spec.require_paths = ["lib"]

  spec.metadata = {
    "homepage_uri"    => spec.homepage,
    "source_code_uri" => "https://github.com/nchapman/trafilatura-rs",
    "bug_tracker_uri" => "https://github.com/nchapman/trafilatura-rs/issues",
  }
end
