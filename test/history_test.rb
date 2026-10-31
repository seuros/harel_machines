# frozen_string_literal: true

require 'test_helper'

module HarelMachines
  class HistoryTest < Minitest::Test
    # door
    # |- closed
    # `- open (shallow history)
    #    |- ajar
    #    `- wide -> latched | free
    def setup
      @chart = Chart.build do |c|
        c.compound :door, initial: :closed
        c.state :closed, parent: :door
        c.compound :open, parent: :door, initial: :ajar, history: :shallow
        c.state :ajar, parent: :open
        c.compound :wide, parent: :open, initial: :latched
        c.state :latched, parent: :wide
        c.state :free, parent: :wide
      end
      @history = History.new(@chart)
    end

    def test_shallow_history_restores_the_child_and_enters_it_at_its_initial
      @history.record(@chart.resolve(%i[free], from: :open, to: :closed))

      step = @chart.resolve(%i[closed], from: :closed, to: :open, history: @history)

      assert_equal %i[open wide latched], step.entry
      assert_equal %i[latched], step.configuration
    end

    def test_without_history_the_initial_child_is_entered_again
      @history.record(@chart.resolve(%i[free], from: :open, to: :closed))

      step = @chart.resolve(%i[closed], from: :closed, to: :open)

      assert_equal %i[open ajar], step.entry
    end

    def test_a_history_that_recorded_nothing_falls_back_to_the_initial_child
      step = @chart.resolve(%i[closed], from: :closed, to: :open, history: @history)

      assert_equal %i[open ajar], step.entry
    end

    def test_entering_from_nothing_reads_history_too
      @history.record(@chart.resolve(%i[free], from: :open, to: :closed))

      assert_equal %i[open wide latched], @chart.enter(:open, history: @history).entry
    end

    def test_deep_history_restores_the_whole_configuration
      chart = Chart.build do |c|
        c.compound :download, initial: :paused
        c.state :paused, parent: :download
        c.compound :running, parent: :download, initial: :transfer, history: :deep
        c.compound :transfer, parent: :running, initial: :headers
        c.state :headers, parent: :transfer
        c.state :body, parent: :transfer
      end
      history = History.new(chart)
      history.record(chart.resolve(%i[body], from: :running, to: :paused))

      step = chart.resolve(%i[paused], from: :paused, to: :running, history: history)

      assert_equal %i[running transfer body], step.entry
      assert_equal %i[body], step.configuration
    end

    def test_deep_history_restores_every_region_of_a_parallel_state
      chart = Chart.build do |c|
        c.compound :radio, initial: :off
        c.state :off, parent: :radio
        c.parallel :on, parent: :radio, history: :deep
        c.compound :volume, parent: :on, initial: :quiet
        c.state :quiet, parent: :volume
        c.state :loud, parent: :volume
        c.compound :link, parent: :on, initial: :online
        c.state :online, parent: :link
        c.state :offline, parent: :link
      end
      history = History.new(chart)
      history.record(chart.resolve(%i[loud offline], from: :on, to: :off))

      step = chart.resolve(%i[off], from: :off, to: :on, history: history)

      assert_equal %i[on volume loud link offline], step.entry
      assert_equal %i[loud offline], step.configuration
    end

    def test_recording_returns_the_history
      step = @chart.resolve(%i[ajar], from: :open, to: :closed)

      assert_same @history, @history.record(step)
    end

    def test_a_history_of_a_larger_chart_reads_no_further_than_this_one
      wide = Chart.build do |c|
        c.compound :root, initial: :first
        c.state :first, parent: :root
        30.times { |index| c.state(:"filler#{index}", parent: :root) }
        c.compound :last, parent: :root, initial: :leaf
        c.state :leaf, parent: :last
      end
      elsewhere = History.new(wide)
      elsewhere.record(wide.resolve(%i[leaf], from: :last, to: :first))

      step = @chart.resolve(%i[closed], from: :closed, to: :open, history: elsewhere)

      assert_equal %i[open ajar], step.entry
    end
  end
end
