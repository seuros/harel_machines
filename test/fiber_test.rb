# frozen_string_literal: true

require 'test_helper'

module HarelMachines
  class FiberTest < Minitest::Test
    REGIONS = SHAPES.max - 1

    def test_the_largest_chart_builds_and_resolves_on_a_fiber_stack
      resolved = Fiber.new do
        chart = Chart.build do |c|
          c.parallel :all
          REGIONS.times { |index| c.state(:"region#{index}", parent: :all) }
        end
        entry = chart.enter(:all)
        step = chart.resolve(entry.configuration, from: :region0, to: :region1)
        [entry, step]
      end.resume
      entry, step = resolved

      assert_equal SHAPES.max, entry.entry.length
      assert_equal REGIONS, entry.configuration.length
      assert_equal REGIONS, step.exit.length

      # The transition domain is the parallel root, so every region it exited
      # has to come back, not only the target's.
      assert_equal REGIONS, step.entry.length
      assert_equal REGIONS, step.configuration.length
      assert_includes step.configuration, :region1
    end
  end
end
