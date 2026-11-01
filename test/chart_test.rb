# frozen_string_literal: true

require 'test_helper'

module HarelMachines
  class ChartTest < Minitest::Test
    # machine
    # |- off
    # `- on
    #    |- idle
    #    |- busy
    #    `- running (parallel)
    #       |- left  -> reading | writing
    #       `- right -> up | down
    def setup
      @chart = Chart.build do |c|
        c.compound :machine, initial: :off
        c.state :off, parent: :machine
        c.compound :on, parent: :machine, initial: :idle
        c.state :idle, parent: :on
        c.state :busy, parent: :on
        c.parallel :running, parent: :on
        c.compound :left, parent: :running, initial: :reading
        c.state :reading, parent: :left
        c.state :writing, parent: :left
        c.compound :right, parent: :running, initial: :up
        c.state :up, parent: :right
        c.state :down, parent: :right
      end
    end

    def test_states_come_back_in_declaration_order
      assert_equal %i[machine off on idle busy running left reading writing right up down], @chart.states
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

    def test_entering_a_compound_state_descends_to_a_leaf
      entry = @chart.enter(:on)

      assert_equal %i[on idle], entry.entry
      assert_empty entry.exit
      assert_equal %i[idle], entry.configuration
    end

    def test_entering_a_parallel_state_activates_every_region
      entry = @chart.enter(:running)

      assert_equal %i[running left reading right up], entry.entry
      assert_equal %i[reading up], entry.configuration
    end

    def test_a_sibling_transition_only_reaches_the_shared_parent
      step = @chart.resolve(%i[idle], from: :idle, to: :busy)

      assert_equal %i[idle], step.exit
      assert_equal %i[busy], step.entry
      assert_equal %i[busy], step.configuration
    end

    def test_leaving_a_deeply_nested_state_exits_innermost_first
      step = @chart.resolve(%i[reading up], from: :reading, to: :off)

      assert_equal %i[reading up left right running on], step.exit
      assert_equal %i[off], step.entry
      assert_equal %i[off], step.configuration
    end

    def test_an_internal_self_transition_exits_nothing
      step = @chart.resolve(%i[idle], from: :idle, to: :idle, kind: :internal)

      assert_empty step.exit
      assert_empty step.entry
      assert_equal %i[idle], step.configuration
    end

    def test_an_external_self_transition_leaves_and_joins_the_state_again
      step = @chart.resolve(%i[idle], from: :idle, to: :idle)

      assert_equal %i[idle], step.exit
      assert_equal %i[idle], step.entry
    end

    def test_an_external_self_transition_on_a_region_leaves_its_siblings_untouched
      step = @chart.resolve(%i[writing down], from: :left, to: :left)

      assert_equal %i[writing left], step.exit
      assert_equal %i[left reading], step.entry
      assert_equal %i[reading down], step.configuration
    end

    def test_an_external_self_transition_on_a_parallel_state_re_enters_every_region
      step = @chart.resolve(%i[writing down], from: :running, to: :running)

      assert_equal %i[writing down left right running], step.exit
      assert_equal %i[running left reading right up], step.entry
      assert_equal %i[reading up], step.configuration
    end

    def test_an_external_self_transition_on_the_root_re_enters_the_whole_chart
      step = @chart.resolve(%i[writing down], from: :machine, to: :machine)

      assert_equal %i[writing down left right running on machine], step.exit
      assert_equal %i[machine off], step.entry
      assert_equal %i[off], step.configuration
    end

    def test_a_transition_to_an_ancestor_exits_the_source_but_not_the_ancestor
      step = @chart.resolve(%i[reading up], from: :reading, to: :on)

      assert_equal %i[reading up left right running], step.exit
      refute_includes step.exit, :on
      assert_equal %i[idle], step.entry
    end

    def test_a_transition_is_frozen
      assert_predicate @chart.resolve(%i[idle], from: :idle, to: :busy), :frozen?
    end

    def test_states_in_disjoint_trees_share_no_ancestor
      chart = Chart.build do |c|
        c.state :here
        c.state :there
      end

      assert_raises(NoCommonAncestor) { chart.least_common_ancestor(:here, :there) }
      assert_raises(NoCommonAncestor) { chart.resolve(%i[here], from: :here, to: :there) }
    end

    def test_a_parallel_state_with_no_regions_reaches_no_leaf
      chart = Chart.build { |c| c.parallel :nothing }

      assert_raises(EmptyParallel) { chart.enter(:nothing) }
    end

    def test_every_chart_error_is_a_harel_machines_error
      assert_operator NoCommonAncestor, :<, Error
      assert_operator EmptyParallel, :<, Error
      assert_operator NotATree, :<, Error
    end
  end
end
