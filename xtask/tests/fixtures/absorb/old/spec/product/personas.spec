// A persona with its status and the features it is for.

use "product/features"

persona writer "Writer" {
  description     "Someone who records short entries every day."
  technical_level intermediate
  status          active
  goals           ["Never lose an entry"]
  pain_points     ["Editors that forget unsaved text"]
  tags            ["primary"]
  key_features    [store_entries]
}
