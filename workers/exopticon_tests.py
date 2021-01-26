import unittest

import exopticon

class TestExopticon(unittest.TestCase):
    def test_narrow_image(self):
        # [height, width]
        dims = [300, 200]
        region = [50, 50, 55, 55]

        selection = exopticon.AnalysisFrame.calculate_region(dims, region, 300)
        self.assertEqual(selection, [0, 0, 199, 299])

    def test_short_image(self):
        # [height, width]
        dims = [200, 300]
        region = [50, 50, 55, 55]

        selection = exopticon.AnalysisFrame.calculate_region(dims, region, 300)
        self.assertEqual(selection, [0, 0, 299, 199])


    def test_left_region(self):
        # [height, width]
        dims = [320, 640]
        region = [0, 0, 49, 639]

        selection = exopticon.AnalysisFrame.calculate_region(dims, region, 300)
        self.assertEqual(selection, [0, 0, 299, 639])

    def test_upper_region(self):
        # [height, width]
        dims = [320, 640]
        region = [0, 3, 299, 63]

        selection = exopticon.AnalysisFrame.calculate_region(dims, region, 300)
        self.assertEqual(selection, [0, 0, 299, 299])

    def test_lower_region(self):
        # [height, width]
        dims = [320, 640]
        region = [0, 280, 299, 319]

        selection = exopticon.AnalysisFrame.calculate_region(dims, region, 300)
        self.assertEqual(selection, [0, 20, 299, 319])


if __name__ == '__main__':
    unittest.main()
