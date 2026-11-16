# frozen_string_literal: true

require_relative 'test_helper'
require 'open3'
require 'rbconfig'

# The classes of the values the extension wraps (Chart, Chart::Declaration,
# History) are fixed when it loads. Nothing a caller does later, redefining a
# constant or `Object.const_get`, reaches an object being built. Each case
# runs in a fresh process: the class is fixed once per process, at load.
class ClassLookupTest < Minitest::Test
  LIB = File.expand_path('../lib', __dir__)

  PRELUDE = <<~RUBY
    require 'harel_machines'
    H = HarelMachines
    BUILD = proc do |c|
      c.compound :root, parent: nil, initial: :idle
      c.atomic :idle, parent: :root
      c.atomic :busy, parent: :root
    end
  RUBY

  # Runs `body` after PRELUDE in a new Ruby; answers its last line of output.
  def isolated(body)
    out, err, status = Open3.capture3(RbConfig.ruby, '-I', LIB, '-e', PRELUDE + body)
    assert status.success?, "exited #{status.exitstatus}: #{err}"
    out.lines.last&.chomp
  end

  # Bug: magnus looked the Chart class up by name on the first build, so a
  # Chart constant redefined before it made the chart an instance of the new
  # class, which no method of the gem accepts.
  def test_a_redefined_chart_constant_does_not_change_the_chart_class
    assert_equal 'true [:idle]', isolated(<<~RUBY)
      original = H::Chart
      H.send(:remove_const, :Chart)
      H.const_set(:Chart, Class.new)
      chart = original.build(&BUILD)
      puts [chart.instance_of?(original), chart.enter(:root).configuration.inspect].join(' ')
    RUBY
  end

  # Bug: the same for a History, which then failed its own exact read.
  def test_a_redefined_history_constant_does_not_change_the_history_class
    assert_equal 'true recorded', isolated(<<~RUBY)
      chart = H::Chart.build(&BUILD)
      original = H::History
      H.send(:remove_const, :History)
      H.const_set(:History, Class.new)
      history = original.new(chart)
      puts [history.instance_of?(original), history.record(chart.resolve([:idle], :idle, :busy, :external))].join(' ')
    RUBY
  end

  # Bug: building the first chart, declaration and history ran
  # `Object.const_get`, a Ruby method a caller can redefine.
  def test_building_runs_no_const_get
    assert_equal '0 Chart History', isolated(<<~RUBY)
      hits = 0
      Object.define_singleton_method(:const_get) do |*args|
        hits += 1
        super(*args)
      end
      chart = H::Chart.build(&BUILD)
      history = H::History.new(chart)
      puts [hits, chart.class.name.split('::').last, history.class.name.split('::').last].join(' ')
    RUBY
  end

  # Bug: a redefined `Object.const_get` that raised hit an `unwrap` in
  # magnus: an uncatchable fatal, and the process exited.
  def test_a_raising_const_get_does_not_kill_the_process
    assert_equal 'recorded', isolated(<<~RUBY)
      Object.define_singleton_method(:const_get) { |*| raise IOError, 'lookup ran mid-call' }
      chart = H::Chart.build(&BUILD)
      puts H::History.new(chart).record(chart.resolve([:idle], :idle, :busy, :external))
    RUBY
  end
end
