# frozen_string_literal: true

require_relative 'harel_machines/version'

# Chart, Chart::Declaration, History, the Transition result object and every
# error class are defined by the native extension; there is no pure-Ruby
# implementation to fall back to.
require 'harel_machines_native/harel_machines_native'
