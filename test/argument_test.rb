# frozen_string_literal: true

require_relative 'test_helper'

# Malformed input raises: TypeError for the wrong type, RangeError for a
# value the chart does not hold, ArgumentError only for arity and keywords.
class ArgumentTest < Minitest::Test
  include ChartHelpers

  def setup
    @chart = H::Chart.build do |c|
      c.compound :on, parent: nil, initial: :idle
      c.atomic :idle, parent: :on
      c.atomic :busy, parent: :on
    end
  end

  def test_a_name_declared_twice_or_never_is_out_of_range
    error = assert_raises(RangeError) { H::Chart.build { |c| c.atomic :orphan, parent: :nowhere } }
    assert_equal 'unknown state: :nowhere', error.message
    assert_raises(RangeError) { H::Chart.build { |c| c.compound :on, parent: nil, initial: :nowhere } }
    twice = assert_raises(RangeError) do
      H::Chart.build do |c|
        c.compound :on, parent: nil, initial: :idle
        c.atomic :idle, parent: :on
        c.atomic :idle, parent: :on
      end
    end
    assert_equal 'state :idle is declared twice', twice.message
  end

  def test_a_tree_the_core_refuses_is_out_of_range
    {
      'an atomic state cannot contain other states' => lambda { |c|
        c.atomic :leaf, parent: nil
        c.atomic :buried, parent: :leaf
      },
      'an initial state is not a child of its compound state' => lambda { |c|
        c.compound :p, parent: nil, initial: :q
        c.compound :q, parent: :p, initial: :p
      },
      'a parallel state has no regions' => ->(c) { c.parallel :nothing, parent: nil },
      'a chart has exactly one root' => lambda { |c|
        c.atomic :here, parent: nil
        c.atomic :there, parent: nil
      },
      'a chart needs at least one state' => ->(_) {}
    }.each do |message, body|
      error = assert_raises(RangeError) { H::Chart.build(&body) }
      assert_equal "not a chart: #{message}", error.message
    end
  end

  def test_names_are_symbols
    assert_raises(TypeError) { H::Chart.build { |c| c.atomic 'root', parent: nil } }
    assert_raises(TypeError) { H::Chart.build { |c| c.atomic :root, parent: false } }
    assert_raises(TypeError) { @chart.depth('idle') }
    assert_raises(TypeError) { @chart.resolve(%w[idle], :idle, :busy, :external) }
    assert_raises(TypeError) { @chart.resolve(%i[idle], :idle, 'busy', :external) }
  end

  def test_a_tree_query_rejects_an_unknown_state
    assert_raises(RangeError) { @chart.depth(:nowhere) }
    assert_raises(RangeError) { @chart.parent(:nowhere) }
    assert_raises(RangeError) { @chart.ancestor?(:on, :nowhere) }
    assert_raises(RangeError) { @chart.least_common_ancestor(:on, :nowhere) }
    assert_raises(RangeError) { @chart.enter(:nowhere) }
  end

  def test_resolving_rejects_an_unknown_source_target_or_leaf
    assert_raises(RangeError) { @chart.resolve(%i[idle], :nowhere, :busy, :external) }
    assert_raises(RangeError) { @chart.resolve(%i[idle], :idle, :nowhere, :external) }
    assert_raises(RangeError) { @chart.resolve(%i[nowhere], :idle, :busy, :external) }
  end

  def test_a_configuration_the_chart_cannot_be_in_is_out_of_range
    assert_raises(RangeError) { @chart.resolve(%i[idle busy], :idle, :busy, :external) }
    assert_raises(RangeError) { @chart.resolve(%i[on], :on, :busy, :external) }
    assert_raises(RangeError) { @chart.resolve(%i[idle idle], :idle, :busy, :external) }
    assert_raises(RangeError) { @chart.configuration(%i[busy idle]) }
  end

  def test_a_configuration_is_exactly_an_array
    assert_raises(TypeError) { @chart.resolve(Class.new(Array).new([:idle]), :idle, :busy, :external) }
    assert_raises(TypeError) { @chart.resolve(nil, :idle, :busy, :external) }
    assert_raises(TypeError) { @chart.configuration(Set[:idle]) }
  end

  def test_more_states_than_the_largest_shape_is_unsupported
    largest = H::SHAPES.max
    error = assert_raises(RangeError) do
      H::Chart.build do |c|
        c.compound :root, parent: nil, initial: :state0
        largest.times { |index| c.atomic(:"state#{index}", parent: :root) }
      end
    end

    assert_equal "a chart of #{largest + 1} states is not supported, the shapes are #{H::SHAPES}", error.message
  end

  def test_a_history_marker_or_kind_outside_the_core_s_is_out_of_range
    error = assert_raises(RangeError) do
      H::Chart.build { |c| c.compound :on, parent: nil, initial: :on, history: :sideways }
    end
    assert_equal 'history must be one of :none, :shallow, :deep', error.message
    assert_raises(TypeError) { H::Chart.build { |c| c.parallel :on, parent: nil, history: 'deep' } }
    kind = assert_raises(RangeError) { @chart.resolve(%i[idle], :idle, :busy, :sideways) }
    assert_equal 'kind must be one of :external, :internal', kind.message
    assert_raises(TypeError) { @chart.resolve(%i[idle], :idle, :busy, nil) }
  end

  def test_arity_and_keywords_are_argument_errors
    assert_raises(ArgumentError) { @chart.resolve(%i[idle], :idle, :busy) }
    assert_raises(ArgumentError) { H::Chart.build { |c| c.atomic :root } }
    assert_raises(ArgumentError) { H::Chart.build { |c| c.atomic :root, parent: nil, history: :deep } }
    assert_raises(LocalJumpError) { H::Chart.build }
  end
end
