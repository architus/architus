__ALL_FEATURE_KEY = 'all'


def get_enabled_components(features_to_components, enabled_features_list):
    """
    Resolves a list of enabled 'features' to the set of 'components' that should be enabled.
    This allows higher-level 'features' to map onto lower-level 'components'
    that are depended on by potentially more than one feature.

    'all' is a special built-in feature that causes all other features
    to be considered enabled no matter what the other enabled features are.

    Arguments:
    - `features_to_components` - a dictionary mapping feature names
    to the list of components that they require.
    - `enabled_features_list` - a list of all enabled feature names,
    which can also include `'all'` as described earlier.

    Returns:
    A "set" (dictionary where each value is `True`) of all enabled components.
    """

    # Use a dict of key -> True as a set
    enabled_features = {f: True for f in enabled_features_list}

    # Check for invalid features
    invalid_features = []
    for feature in enabled_features:
        if feature != __ALL_FEATURE_KEY and feature not in features_to_components:
            invalid_features.push(feature)
    if invalid_features:
        valid_features = [f for f in features_to_components] + [__ALL_FEATURE_KEY]
        fail("Invalid features specified: " + repr(invalid_features) + "\n"
            + "  Given: " + repr(enabled_features_list) + "\n"
            + "  Valid features: " + repr(valid_features))

    # Use all features if 'all' was specified
    if __ALL_FEATURE_KEY in enabled_features:
        enabled_features = {f: True for f in features_to_components}

    enabled = {}
    for f in enabled_features:
        for component in features_to_components[f]:
            enabled[component] = True

    return enabled
