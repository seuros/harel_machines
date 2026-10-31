# frozen_string_literal: true

require_relative 'lib/harel_machines/version'

Gem::Specification.new do |spec|
  spec.name = 'harel_machines'
  spec.version = HarelMachines::VERSION
  spec.authors = ['Abdelkader Boudih']
  spec.email = ['oss@seuros.com']

  spec.summary = 'Statechart hierarchy for Ruby: which states a transition exits and enters'
  spec.description = 'A flat finite state machine answers which state comes next. This answers what ' \
                     'happens to the states it was inside of: the least common ancestor of a source ' \
                     'and a target, the exit set innermost first, the entry set outermost first, ' \
                     'parallel regions and history. Bound to Ruby from the no_std harel-machines crate: ' \
                     'the tree algebra only, with events, guards and callbacks left to state_machines.'
  spec.homepage = 'https://github.com/seuros/harel_machines'
  spec.license = 'MIT'
  spec.required_ruby_version = '>= 4.0.0'

  spec.metadata['source_code_uri'] = spec.homepage
  spec.metadata['changelog_uri'] = "#{spec.homepage}/blob/main/CHANGELOG.md"
  spec.metadata['bug_tracker_uri'] = "#{spec.homepage}/issues"
  spec.metadata['rubygems_mfa_required'] = 'true'
  spec.metadata['cargo_crate_name'] = 'harel_machines_native'
  spec.metadata['cargo_manifest_path'] = 'ext/harel_machines_native/ffi/Cargo.toml'

  spec.files = Dir.glob(%w[
                          lib/**/*.rb
                          sig/**/*.rbs
                          ext/**/*.{rb,rs,toml}
                          Cargo.toml
                          Cargo.lock
                          README.md
                          CHANGELOG.md
                          LICENSE.txt
                        ]).select { |path| File.exist?(path) }
                   .reject { |path| path =~ %r{/tests(/|\.rs\z)} }
  spec.require_paths = ['lib']
  spec.extensions = ['ext/harel_machines_native/extconf.rb']

  spec.add_development_dependency 'rb_sys', '~> 0.9'
end
