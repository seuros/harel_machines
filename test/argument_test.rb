# frozen_string_literal: true

require 'test_helper'

module HarelMachines
  class ArgumentTest < Minitest::Test
    def setup
      @chart = Chart.build do |c|
        c.compound :on, initial: :idle
        c.state :idle, parent: :on
        c.state :busy, parent: :on
      end
    end

    def test_a_parent_that_was_never_declared_is_unknown
      error = assert_raises(ArgumentError) do
        Chart.build do |c|
          c.state :orphan, parent: :nowhere
        end
      end

      assert_equal 'unknown state: :nowhere', error.message
    end

    def test_an_initial_child_that_was_never_declared_is_unknown
      assert_raises(ArgumentError) do
        Chart.build { |c| c.compound :on, initial: :nowhere }
      end
    end

    def test_an_initial_child_that_is_not_a_child_is_rejected
      error = assert_raises(ArgumentError) do
        Chart.build do |c|
          c.compound :p, initial: :outside
          c.state :outside
        end
      end

      assert_equal 'initial state :outside is not a child of :p', error.message
    end

    def test_a_tree_query_rejects_an_unknown_state
      assert_raises(ArgumentError) { @chart.depth(:nowhere) }
      assert_raises(ArgumentError) { @chart.ancestor?(:on, :nowhere) }
      assert_raises(ArgumentError) { @chart.least_common_ancestor(:on, :nowhere) }
      assert_raises(ArgumentError) { @chart.enter(:nowhere) }
    end

    def test_resolving_rejects_an_unknown_source_target_or_leaf
      assert_raises(ArgumentError) { @chart.resolve(%i[idle], from: :nowhere, to: :busy) }
      assert_raises(ArgumentError) { @chart.resolve(%i[idle], from: :idle, to: :nowhere) }
      assert_raises(ArgumentError) { @chart.resolve(%i[nowhere], from: :idle, to: :busy) }
    end

    def test_more_states_than_the_largest_shape_is_unsupported
      largest = SHAPES.max
      error = assert_raises(ArgumentError) do
        Chart.build do |c|
          c.compound :root, initial: :state0
          largest.times { |index| c.state(:"state#{index}", parent: :root) }
        end
      end

      assert_equal "a chart of #{largest + 1} states is not supported, the shapes are #{SHAPES}", error.message
    end

    def test_a_history_marker_outside_shallow_and_deep_is_unknown
      error = assert_raises(ArgumentError) do
        Chart.build { |c| c.compound :on, initial: :on, history: :sideways }
      end

      assert_equal 'unknown history: :sideways, expected :shallow or :deep', error.message
    end

    def test_a_transition_kind_outside_external_and_internal_is_unknown
      error = assert_raises(ArgumentError) do
        @chart.resolve(%i[idle], from: :idle, to: :busy, kind: :sideways)
      end

      assert_equal 'unknown transition kind: :sideways, expected :external or :internal', error.message
    end
  end
end
