# frozen_string_literal: true

require 'minitest/autorun'
require 'harel_machines'

# The charts the tests share.
module ChartHelpers
  H = HarelMachines

  # machine
  # |- off
  # `- on
  #    |- idle
  #    |- busy
  #    `- running (parallel)
  #       |- left  -> reading | writing
  #       `- right -> up | down
  def machine
    H::Chart.build do |c|
      c.compound :machine, parent: nil, initial: :off
      c.atomic :off, parent: :machine
      c.compound :on, parent: :machine, initial: :idle
      c.atomic :idle, parent: :on
      c.atomic :busy, parent: :on
      c.parallel :running, parent: :on
      c.compound :left, parent: :running, initial: :reading
      c.atomic :reading, parent: :left
      c.atomic :writing, parent: :left
      c.compound :right, parent: :running, initial: :up
      c.atomic :up, parent: :right
      c.atomic :down, parent: :right
    end
  end

  # door
  # |- closed
  # `- open (history)
  #    |- ajar
  #    `- wide -> latched | free
  def door(history = :shallow)
    H::Chart.build do |c|
      c.compound :door, parent: nil, initial: :closed
      c.atomic :closed, parent: :door
      c.compound :open, parent: :door, initial: :ajar, history: history
      c.atomic :ajar, parent: :open
      c.compound :wide, parent: :open, initial: :latched
      c.atomic :latched, parent: :wide
      c.atomic :free, parent: :wide
    end
  end
end
