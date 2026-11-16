# frozen_string_literal: true

require_relative 'test_helper'

class ChartTest < Minitest::Test
  include ChartHelpers

  def setup
    @chart = machine
  end

  def test_the_tree_reads_back
    assert_equal %i[machine off on idle busy running left reading writing right up down], @chart.states
    assert_predicate @chart.states, :frozen?
    assert_equal 12, @chart.size
    assert_equal :machine, @chart.root
    assert_equal :running, @chart.parent(:left)
    assert_nil @chart.parent(:machine)
  end

  def test_depth_counts_the_states_that_contain_one
    assert_equal 0, @chart.depth(:machine)
    assert_equal 2, @chart.depth(:idle)
    assert_equal 4, @chart.depth(:reading)
  end

  def test_a_state_contains_what_is_nested_under_it_but_not_itself
    assert @chart.ancestor?(:on, :reading)
    refute @chart.ancestor?(:on, :on)
    refute @chart.ancestor?(:left, :up)
  end

  def test_the_least_common_ancestor_of_two_states_is_what_holds_both
    assert_equal :on, @chart.least_common_ancestor(:idle, :busy)
    assert_equal :running, @chart.least_common_ancestor(:reading, :down)
    assert_equal :on, @chart.least_common_ancestor(:on, :reading)
  end

  def test_entering_from_nothing_enters_every_state_from_the_root_down
    entry = @chart.enter(:on)

    assert_equal %i[machine on idle], entry.entry
    assert_empty entry.exit
    assert_equal %i[idle], entry.configuration
  end

  def test_entering_a_parallel_state_activates_every_region
    entry = @chart.enter(:running)

    assert_equal %i[machine on running left reading right up], entry.entry
    assert_equal %i[reading up], entry.configuration
  end

  def test_a_sibling_transition_only_reaches_the_shared_parent
    step = @chart.resolve(%i[idle], :idle, :busy, :external)

    assert_equal %i[idle], step.exit
    assert_equal %i[busy], step.entry
    assert_equal %i[busy], step.configuration
  end

  def test_leaving_a_deeply_nested_state_exits_innermost_first
    step = @chart.resolve(%i[reading up], :reading, :off, :external)

    assert_equal %i[reading up left right running on], step.exit
    assert_equal %i[off], step.entry
    assert_equal %i[off], step.configuration
  end

  def test_an_internal_self_transition_exits_nothing
    step = @chart.resolve(%i[idle], :idle, :idle, :internal)

    assert_empty step.exit
    assert_empty step.entry
    assert_equal %i[idle], step.configuration
  end

  def test_an_external_self_transition_leaves_and_joins_the_state_again
    step = @chart.resolve(%i[idle], :idle, :idle, :external)

    assert_equal %i[idle], step.exit
    assert_equal %i[idle], step.entry
  end

  def test_an_external_self_transition_on_a_region_leaves_its_siblings_untouched
    step = @chart.resolve(%i[writing down], :left, :left, :external)

    assert_equal %i[writing left], step.exit
    assert_equal %i[left reading], step.entry
    assert_equal %i[reading down], step.configuration
  end

  def test_an_external_self_transition_on_a_parallel_state_re_enters_every_region
    step = @chart.resolve(%i[down writing], :running, :running, :external)

    assert_equal %i[writing down left right running], step.exit
    assert_equal %i[running left reading right up], step.entry
    assert_equal %i[reading up], step.configuration
  end

  def test_an_external_self_transition_on_the_root_re_enters_the_whole_chart
    step = @chart.resolve(%i[writing down], :machine, :machine, :external)

    assert_equal %i[writing down left right running on machine], step.exit
    assert_equal %i[machine off], step.entry
    assert_equal %i[off], step.configuration
  end

  def test_a_transition_to_an_ancestor_exits_the_source_but_not_the_ancestor
    step = @chart.resolve(%i[reading up], :reading, :on, :external)

    assert_equal %i[reading up left right running], step.exit
    assert_equal %i[idle], step.entry
  end

  def test_a_transition_and_its_arrays_are_frozen
    step = @chart.resolve(%i[idle], :idle, :busy, :external)

    assert_predicate step, :frozen?
    [step.exit, step.entry, step.configuration].each { |list| assert_predicate list, :frozen? }
  end

  def test_a_source_that_is_not_active_is_refused_as_a_symbol
    assert_equal :inactive, @chart.resolve(%i[idle], :busy, :off, :external)
    assert_equal :inactive, @chart.resolve([], :machine, :off, :external)
    assert_equal %i[unknown_state invalid_configuration invalid_history inactive], H::REFUSALS
    assert_predicate H::REFUSALS, :frozen?
  end

  def test_a_configuration_reads_back_in_declaration_order
    assert_equal %i[reading up], @chart.configuration(%i[up reading])
    assert_equal [], @chart.configuration([])
    assert_predicate @chart.configuration(%i[idle]), :frozen?
  end

  def test_the_largest_chart_builds_and_resolves_on_a_fiber_stack
    regions = H::SHAPES.max - 1
    entry, step = Fiber.new do
      chart = H::Chart.build do |c|
        c.parallel :all, parent: nil
        regions.times { |index| c.atomic(:"region#{index}", parent: :all) }
      end
      entry = chart.enter(:all)
      [entry, chart.resolve(entry.configuration, :region0, :region1, :external)]
    end.resume

    assert_equal H::SHAPES.max, entry.entry.length
    assert_equal regions, entry.configuration.length
    # The transition domain is the parallel root, so every region it exited
    # has to come back, not only the target's.
    assert_equal regions, step.exit.length
    assert_equal regions, step.entry.length
    assert_includes step.configuration, :region1
  end
end

