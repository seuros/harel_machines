# frozen_string_literal: true

require_relative 'test_helper'
require 'fileutils'
require 'open3'
require 'tmpdir'

class PackagingTest < Minitest::Test
  include ChartHelpers

  # The classes the extension holds are pinned: a compaction moves nothing
  # they point at, and building after one still works.
  def test_charts_and_histories_build_after_gc_compact
    chart = door
    history = H::History.new(chart)
    GC.compact
    step = machine.resolve(%i[idle], :idle, :busy, :external)

    assert_instance_of H::Transition, step
    assert_equal :recorded, history.record(chart.resolve(%i[free], :open, :closed, :external))
    assert_instance_of H::History, H::History.restored(chart, history.remembered)
    assert_nil chart.method(:resolve).source_location
  end

  def test_a_missing_extension_raises_load_error_instead_of_using_a_fallback
    Dir.mktmpdir('harel-loader-') do |directory|
      FileUtils.cp_r(File.expand_path('../lib/harel_machines', __dir__), directory)
      FileUtils.cp(File.expand_path('../lib/harel_machines.rb', __dir__), directory)
      code = <<~RUBY
        begin
          require 'harel_machines'
          abort 'unexpected fallback'
        rescue LoadError => error
          abort error.message unless error.path == 'harel_machines_native/harel_machines_native'
        end
      RUBY
      _stdout, stderr, status = Open3.capture3(
        { 'RUBYOPT' => nil, 'RUBYLIB' => nil },
        RbConfig.ruby, '--disable-gems', "-I#{directory}", '-e', code
      )
      assert status.success?, stderr
    end
  end
end
